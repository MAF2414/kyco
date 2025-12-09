package com.kyco.plugin

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.project.Project
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.ui.Messages
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.progress.Task
import com.intellij.psi.search.FilenameIndex
import com.intellij.psi.search.GlobalSearchScope
import com.intellij.openapi.application.ReadAction
import com.intellij.openapi.vfs.VirtualFile
import java.net.HttpURLConnection
import java.net.URI
import java.nio.charset.StandardCharsets
import com.google.gson.Gson
import com.intellij.openapi.application.ApplicationManager
import java.util.regex.Pattern

/**
 * Action to search for files matching a pattern and send them to KYCo as a batch.
 */
class SendGrepAction : AnAction() {

    private data class BatchFile(
        val path: String,
        val workspace: String,
        val line_start: Int?,
        val line_end: Int?
    )

    private data class BatchPayload(
        val files: List<BatchFile>
    )

    override fun actionPerformed(event: AnActionEvent) {
        val project = event.project ?: return

        // Ask user for search pattern
        val pattern = Messages.showInputDialog(
            project,
            "Enter search pattern (regex supported):",
            "KYCo: Search & Send",
            Messages.getQuestionIcon(),
            "",
            null
        )

        if (pattern.isNullOrBlank()) {
            return // User cancelled
        }

        // Ask for file extension filter (optional)
        val fileExtension = Messages.showInputDialog(
            project,
            "File extension filter (leave empty for all files):",
            "KYCo: File Filter",
            Messages.getQuestionIcon(),
            "",
            null
        )

        if (fileExtension == null) {
            return // User cancelled
        }

        // Run search in background with progress
        ProgressManager.getInstance().run(object : Task.Backgroundable(project, "KYCo: Searching files...", true) {
            override fun run(indicator: ProgressIndicator) {
                try {
                    indicator.isIndeterminate = false
                    indicator.fraction = 0.0

                    val regex = Pattern.compile(pattern)
                    val matchingFiles = mutableListOf<VirtualFile>()
                    val workspace = project.basePath ?: ""

                    // Get all files in project
                    val allFiles = ReadAction.compute<Collection<VirtualFile>, Throwable> {
                        val scope = GlobalSearchScope.projectScope(project)
                        if (fileExtension.isNotBlank()) {
                            FilenameIndex.getAllFilesByExt(project, fileExtension, scope)
                        } else {
                            // Get all files - use a common approach
                            val files = mutableListOf<VirtualFile>()
                            FilenameIndex.processAllFileNames({ name ->
                                val matchedFiles = FilenameIndex.getVirtualFilesByName(name, scope)
                                files.addAll(matchedFiles)
                                true
                            }, scope, null)
                            files.distinctBy { it.path }
                        }
                    }

                    val fileList = allFiles.toList()
                    val totalFiles = fileList.size

                    // Search for pattern in files
                    for ((index, file) in fileList.withIndex()) {
                        if (indicator.isCanceled) {
                            return
                        }

                        indicator.fraction = index.toDouble() / totalFiles
                        indicator.text2 = "Checking ${file.name}..."

                        // Skip binary files and large files
                        if (file.length > 1_000_000) continue // Skip files > 1MB

                        try {
                            val content = ReadAction.compute<String, Throwable> {
                                String(file.contentsToByteArray(), Charsets.UTF_8)
                            }

                            if (regex.matcher(content).find()) {
                                matchingFiles.add(file)
                            }
                        } catch (e: Exception) {
                            // Skip files that can't be read
                        }
                    }

                    indicator.fraction = 1.0

                    if (matchingFiles.isEmpty()) {
                        ApplicationManager.getApplication().invokeLater {
                            showNotification(project, "No files matching \"$pattern\" found", NotificationType.INFORMATION)
                        }
                        return
                    }

                    // Confirm with user
                    ApplicationManager.getApplication().invokeLater {
                        val confirm = Messages.showYesNoDialog(
                            project,
                            "Found ${matchingFiles.size} files matching \"$pattern\". Send to KYCo?",
                            "KYCo: Confirm",
                            Messages.getQuestionIcon()
                        )

                        if (confirm == Messages.YES) {
                            // Send in background
                            ApplicationManager.getApplication().executeOnPooledThread {
                                val batchFiles = matchingFiles.map { file ->
                                    BatchFile(
                                        path = file.path,
                                        workspace = workspace,
                                        line_start = null,
                                        line_end = null
                                    )
                                }

                                val payload = BatchPayload(files = batchFiles)
                                sendRequest(project, payload, matchingFiles.size)
                            }
                        }
                    }

                } catch (e: Exception) {
                    ApplicationManager.getApplication().invokeLater {
                        showNotification(project, "Search failed: ${e.message}", NotificationType.ERROR)
                    }
                }
            }
        })
    }

    private fun sendRequest(project: Project?, payload: BatchPayload, fileCount: Int) {
        try {
            val url = URI("http://localhost:9876/batch").toURL()
            val connection = url.openConnection() as HttpURLConnection

            connection.requestMethod = "POST"
            connection.setRequestProperty("Content-Type", "application/json")
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
                    showNotification(project, "Grep batch sent ($fileCount files)", NotificationType.INFORMATION)
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
        // Only enable when a project is open
        event.presentation.isEnabledAndVisible = event.project != null
    }
}
