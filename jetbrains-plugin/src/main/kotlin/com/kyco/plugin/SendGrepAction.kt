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
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.vfs.LocalFileSystem
import java.net.HttpURLConnection
import java.net.URI
import java.nio.charset.StandardCharsets
import com.google.gson.Gson
import com.intellij.openapi.application.ApplicationManager
import java.io.File
import java.util.concurrent.TimeUnit

/**
 * Action to search for files matching a pattern and send them to KYCo as a batch.
 * Uses external tools (rg/grep) for maximum search performance.
 */
class SendGrepAction : AnAction() {

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

        // Ask for file glob/extension filter (optional)
        val fileFilter = Messages.showInputDialog(
            project,
            "File filter (e.g., *.kt, *.java - leave empty for all):",
            "KYCo: File Filter",
            Messages.getQuestionIcon(),
            "",
            null
        )

        if (fileFilter == null) {
            return // User cancelled
        }

        val workspace = project.basePath ?: return

        // Run search in background with progress
        ProgressManager.getInstance().run(object : Task.Backgroundable(project, "KYCo: Searching files...", true) {
            override fun run(indicator: ProgressIndicator) {
                try {
                    indicator.isIndeterminate = true
                    indicator.text = "Searching with external tool..."

                    // Try rg first, then grep
                    val matchingFilePaths = searchWithExternalTool(pattern, fileFilter, workspace, indicator)

                    if (indicator.isCanceled) {
                        return
                    }

                    if (matchingFilePaths.isEmpty()) {
                        ApplicationManager.getApplication().invokeLater {
                            showNotification(project, "No files matching \"$pattern\" found", NotificationType.INFORMATION)
                        }
                        return
                    }

                    // Convert paths to VirtualFiles for git_root/project_root resolution
                    val localFileSystem = LocalFileSystem.getInstance()
                    val matchingFiles = matchingFilePaths.mapNotNull { path ->
                        localFileSystem.findFileByPath(path)
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
                                        git_root = SendSelectionAction.getGitRoot(project, file),
                                        project_root = SendSelectionAction.getProjectRoot(project, file),
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

    /**
     * Search using external tools: tries rg first, then grep.
     * Returns list of absolute file paths.
     */
    private fun searchWithExternalTool(
        pattern: String,
        fileFilter: String,
        cwd: String,
        indicator: ProgressIndicator
    ): List<String> {
        // Try ripgrep first (fastest)
        if (commandExists("rg")) {
            indicator.text = "Searching with ripgrep..."
            return searchWithRipgrep(pattern, fileFilter, cwd)
        }

        // Fallback to grep on Unix systems
        if (!System.getProperty("os.name").lowercase().contains("win") && commandExists("grep")) {
            indicator.text = "Searching with grep..."
            return searchWithGrep(pattern, fileFilter, cwd)
        }

        // No external tool available
        throw RuntimeException("Neither 'rg' (ripgrep) nor 'grep' found. Please install ripgrep for best performance.")
    }

    /**
     * Check if a command exists on the system
     */
    private fun commandExists(cmd: String): Boolean {
        return try {
            val checkCmd = if (System.getProperty("os.name").lowercase().contains("win")) "where" else "which"
            val process = ProcessBuilder(checkCmd, cmd)
                .redirectErrorStream(true)
                .start()
            process.waitFor(5, TimeUnit.SECONDS)
            process.exitValue() == 0
        } catch (e: Exception) {
            false
        }
    }

    /**
     * Search using ripgrep (rg) - extremely fast, respects .gitignore
     */
    private fun searchWithRipgrep(pattern: String, fileFilter: String, cwd: String): List<String> {
        val args = mutableListOf(
            "rg",
            "--files-with-matches",
            "--no-heading",
            "--color=never",
            "-e", pattern
        )

        if (fileFilter.isNotBlank()) {
            args.add("--glob")
            args.add(fileFilter)
        }

        val process = ProcessBuilder(args)
            .directory(File(cwd))
            .redirectErrorStream(false)
            .start()

        val output = process.inputStream.bufferedReader().readText()
        process.waitFor(60, TimeUnit.SECONDS)

        // rg returns 1 when no matches, which is fine
        return output.lines()
            .filter { it.isNotBlank() }
            .map { line ->
                val file = File(line)
                if (file.isAbsolute) line else File(cwd, line).absolutePath
            }
    }

    /**
     * Search using grep -r (Unix systems)
     */
    private fun searchWithGrep(pattern: String, fileFilter: String, cwd: String): List<String> {
        val args = mutableListOf("grep", "-rlE", pattern)

        if (fileFilter.isNotBlank()) {
            args.add("--include=$fileFilter")
        }

        // Exclude common directories
        args.addAll(listOf(
            "--exclude-dir=node_modules",
            "--exclude-dir=.git",
            "--exclude-dir=target",
            "--exclude-dir=dist",
            "--exclude-dir=build",
            "."
        ))

        val process = ProcessBuilder(args)
            .directory(File(cwd))
            .redirectErrorStream(false)
            .start()

        val output = process.inputStream.bufferedReader().readText()
        process.waitFor(60, TimeUnit.SECONDS)

        return output.lines()
            .filter { it.isNotBlank() }
            .map { line ->
                val cleanPath = if (line.startsWith("./")) line.substring(2) else line
                File(cwd, cleanPath).absolutePath
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
