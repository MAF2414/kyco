package com.kyco.plugin

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.project.Project
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.vfs.VirtualFile
import java.net.HttpURLConnection
import java.net.URI
import java.nio.charset.StandardCharsets
import com.google.gson.Gson
import com.intellij.openapi.application.ApplicationManager

/**
 * Action to send multiple selected files to KYCo as a batch.
 * Works when files are selected in the Project View.
 */
class SendBatchAction : AnAction() {

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

        // Filter to only include files (not directories)
        val files = virtualFiles.filter { !it.isDirectory }

        if (files.isEmpty()) {
            showNotification(project, "No files selected (only directories)", NotificationType.ERROR)
            return
        }

        // Get workspace path
        val workspace = project?.basePath ?: ""

        // Send in background thread
        ApplicationManager.getApplication().executeOnPooledThread {
            val batchFiles = files.map { file ->
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
            sendRequest(project, payload, files.size)
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
        // Only enable when files are selected
        val virtualFiles = event.getData(CommonDataKeys.VIRTUAL_FILE_ARRAY)
        val hasFiles = virtualFiles?.any { !it.isDirectory } == true
        event.presentation.isEnabledAndVisible = hasFiles
    }
}
