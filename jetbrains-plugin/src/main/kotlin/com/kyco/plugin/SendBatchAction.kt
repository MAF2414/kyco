package com.kyco.plugin

import com.intellij.ide.util.PropertiesComponent
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.project.Project
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.ui.Messages
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.vfs.VfsUtilCore
import java.net.HttpURLConnection
import java.net.URI
import java.nio.charset.StandardCharsets
import com.google.gson.Gson
import com.intellij.openapi.application.ApplicationManager
import java.nio.file.FileSystems
import java.nio.file.PathMatcher
import java.nio.file.Paths

/**
 * Action to send multiple selected files to KYCo as a batch.
 * Works when files are selected in the Project View.
 */
class SendBatchAction : AnAction() {

    private fun normalizeSubtreeGlob(glob: String): String {
        val trimmed = glob.trim()
        if (trimmed.isEmpty()) {
            return "**/*"
        }

        // If the user already provided a path-aware glob, keep it as-is.
        if (trimmed.contains('/') || trimmed.contains('\\') || trimmed.startsWith("**")) {
            return trimmed
        }

        return "**/$trimmed"
    }

    private fun shouldSkipDirectory(name: String): Boolean {
        return name == ".git" ||
            name == "node_modules" ||
            name == "target" ||
            name == "dist" ||
            name == "build" ||
            name == "out" ||
            name == ".idea" ||
            name == ".gradle" ||
            name == "__pycache__" ||
            name == ".venv" ||
            name == "venv"
    }

    private fun collectFilesMatchingGlob(directories: List<VirtualFile>, glob: String): List<VirtualFile> {
        val matcher: PathMatcher = FileSystems.getDefault().getPathMatcher("glob:$glob")
        val results = mutableListOf<VirtualFile>()

        for (dir in directories) {
            collectFilesRecursively(root = dir, current = dir, matcher = matcher, results = results)
        }

        return results
    }

    private fun collectFilesRecursively(
        root: VirtualFile,
        current: VirtualFile,
        matcher: PathMatcher,
        results: MutableList<VirtualFile>
    ) {
        for (child in current.children) {
            if (child.isDirectory) {
                if (shouldSkipDirectory(child.name)) {
                    continue
                }
                collectFilesRecursively(root, child, matcher, results)
                continue
            }

            val relativePath = VfsUtilCore.getRelativePath(child, root, '/') ?: child.name
            if (matcher.matches(Paths.get(relativePath))) {
                results.add(child)
            }
        }
    }

    private data class BatchFile(
        val path: String,
        val workspace: String,
        val git_root: String?,
        val project_root: String,
        val line_start: Int?,
        val line_end: Int?
    )

    private data class BatchPayload(
        val files: List<BatchFile>
    )

    override fun actionPerformed(event: AnActionEvent) {
        val project = event.project
        val virtualFiles = event.getData(CommonDataKeys.VIRTUAL_FILE_ARRAY)

        if (virtualFiles == null || virtualFiles.isEmpty()) {
            showNotification(project, "No files selected", NotificationType.ERROR)
            return
        }

        val selectedFiles = virtualFiles.filter { !it.isDirectory }
        val selectedDirs = virtualFiles.filter { it.isDirectory }

        val folderGlob: String? = if (selectedDirs.isNotEmpty()) {
            val defaultGlob = "**/*.{sol,ts,js,tsx,jsx,py}"
            val props = if (project != null) PropertiesComponent.getInstance(project) else PropertiesComponent.getInstance()
            val initial = props.getValue("kyco.sendBatch.folderGlob") ?: defaultGlob
            val raw = Messages.showInputDialog(
                project,
                "Folder file glob (supports **, {}, e.g. **/*.{sol,ts,js,tsx,jsx,py}):",
                "KYCo: Folder Glob",
                Messages.getQuestionIcon(),
                initial,
                null
            ) ?: return // cancelled

            val normalized = normalizeSubtreeGlob(raw)
            props.setValue("kyco.sendBatch.folderGlob", normalized)
            normalized
        } else {
            null
        }

        // Get workspace path
        val workspace = project?.basePath ?: ""

        // Send in background thread
        ApplicationManager.getApplication().executeOnPooledThread {
            val expandedDirFiles = if (folderGlob != null && selectedDirs.isNotEmpty()) {
                collectFilesMatchingGlob(selectedDirs, folderGlob)
            } else {
                emptyList()
            }

            val allFiles = (selectedFiles + expandedDirFiles)
                .distinctBy { it.path }

            if (allFiles.isEmpty()) {
                ApplicationManager.getApplication().invokeLater {
                    showNotification(project, "No matching files found", NotificationType.ERROR)
                }
                return@executeOnPooledThread
            }

            // Confirm when folders were expanded to avoid accidental huge batches.
            if (selectedDirs.isNotEmpty()) {
                var confirm = Messages.NO
                ApplicationManager.getApplication().invokeAndWait {
                    confirm = Messages.showYesNoDialog(
                        project,
                        "Found ${allFiles.size} files. Send to KYCo?",
                        "KYCo: Confirm",
                        Messages.getQuestionIcon()
                    )
                }
                if (confirm != Messages.YES) {
                    return@executeOnPooledThread
                }
            }

            val batchFiles = allFiles.map { file ->
                BatchFile(
                    path = file.path,
                    workspace = workspace,
                    git_root = if (project != null) SendSelectionAction.getGitRoot(project, file) else null,
                    project_root = if (project != null) SendSelectionAction.getProjectRoot(project, file) else workspace,
                    line_start = null,
                    line_end = null
                )
            }

            val payload = BatchPayload(files = batchFiles)
            sendRequest(project, payload, allFiles.size)
        }
    }

    private fun sendRequest(project: Project?, payload: BatchPayload, fileCount: Int) {
        try {
            val url = URI("http://localhost:${KycoHttpAuth.port(payload.files.firstOrNull()?.workspace)}/batch").toURL()
            val connection = url.openConnection() as HttpURLConnection

            connection.requestMethod = "POST"
            connection.setRequestProperty("Content-Type", "application/json")
            KycoHttpAuth.apply(connection, payload.files.firstOrNull()?.workspace)
            connection.doOutput = true
            connection.connectTimeout = 5000
            connection.readTimeout = 5000

            val jsonPayload = Gson().toJson(payload)
            connection.outputStream.use { os ->
                os.write(jsonPayload.toByteArray(StandardCharsets.UTF_8))
            }

            val responseCode = connection.responseCode
            connection.disconnect()

            ApplicationManager.getApplication().invokeLater {
                if (responseCode in 200..299) {
                    showNotification(project, "Batch sent ($fileCount files)", NotificationType.INFORMATION)
                } else {
                    showNotification(project, "Server responded with status $responseCode", NotificationType.ERROR)
                }
            }
        } catch (e: Exception) {
            ApplicationManager.getApplication().invokeLater {
                showNotification(project, "Failed to send batch - ${e.message}", NotificationType.ERROR)
            }
        }
    }

    private fun showNotification(project: Project?, message: String, type: NotificationType) {
        NotificationGroupManager.getInstance()
            .getNotificationGroup("Kyco Notifications")
            .createNotification("Kyco", message, type)
            .notify(project)
    }

    override fun update(event: AnActionEvent) {
        // Enable when files or folders are selected
        val virtualFiles = event.getData(CommonDataKeys.VIRTUAL_FILE_ARRAY)
        val hasSelection = virtualFiles?.isNotEmpty() == true
        event.presentation.isEnabledAndVisible = event.project != null && hasSelection
    }
}
