package com.kyco.plugin

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.project.Project
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import java.net.HttpURLConnection
import java.net.URI
import java.nio.charset.StandardCharsets
import com.google.gson.Gson
import com.intellij.openapi.application.ApplicationManager

class SendSelectionAction : AnAction() {

    private data class SelectionPayload(
        val file_path: String,
        val selected_text: String,
        val line_start: Int,
        val line_end: Int,
        val workspace: String
    )

    override fun actionPerformed(event: AnActionEvent) {
        val project = event.project
        val editor = event.getData(CommonDataKeys.EDITOR)

        if (editor == null) {
            showNotification(project, "No active editor", NotificationType.ERROR)
            return
        }

        val document = editor.document
        val selectionModel = editor.selectionModel
        val virtualFile = event.getData(CommonDataKeys.VIRTUAL_FILE)

        // Get file path
        val filePath = virtualFile?.path ?: ""

        // Get selected text
        val selectedText = selectionModel.selectedText ?: ""

        // Get line numbers (1-indexed)
        val lineStart = document.getLineNumber(selectionModel.selectionStart) + 1
        val lineEnd = document.getLineNumber(selectionModel.selectionEnd) + 1

        // Get workspace path
        val workspace = project?.basePath ?: ""

        val payload = SelectionPayload(
            file_path = filePath,
            selected_text = selectedText,
            line_start = lineStart,
            line_end = lineEnd,
            workspace = workspace
        )

        // Send request in background thread
        ApplicationManager.getApplication().executeOnPooledThread {
            sendRequest(project, payload)
        }
    }

    private fun sendRequest(project: Project?, payload: SelectionPayload) {
        try {
            val url = URI("http://localhost:9876/selection").toURL()
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
                    showNotification(project, "Selection sent successfully", NotificationType.INFORMATION)
                } else {
                    showNotification(project, "Server responded with status $responseCode", NotificationType.ERROR)
                }
            }
        } catch (e: Exception) {
            ApplicationManager.getApplication().invokeLater {
                showNotification(project, "Failed to send selection - ${e.message}", NotificationType.ERROR)
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
        // Only enable when there's an active editor
        val editor = event.getData(CommonDataKeys.EDITOR)
        event.presentation.isEnabledAndVisible = editor != null
    }
}
