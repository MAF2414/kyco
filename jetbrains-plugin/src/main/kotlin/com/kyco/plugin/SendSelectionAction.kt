package com.kyco.plugin

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.project.Project
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import com.intellij.psi.PsiManager
import com.intellij.psi.search.FilenameIndex
import com.intellij.psi.search.GlobalSearchScope
import com.intellij.psi.search.searches.ReferencesSearch
import com.intellij.psi.util.PsiTreeUtil
import com.intellij.openapi.vfs.VirtualFile
import java.net.HttpURLConnection
import java.net.URI
import java.nio.charset.StandardCharsets
import com.google.gson.Gson
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.application.ReadAction

class SendSelectionAction : AnAction() {

    private data class Dependency(
        val file_path: String,
        val line: Int
    )

    private data class SelectionPayload(
        val file_path: String,
        val selected_text: String,
        val line_start: Int,
        val line_end: Int,
        val workspace: String,
        val dependencies: List<Dependency>,
        val dependency_count: Int,
        val additional_dependency_count: Int,
        val related_tests: List<String>
    )

    companion object {
        private const val MAX_DEPENDENCIES = 30
    }

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
        val psiFile = event.getData(CommonDataKeys.PSI_FILE)

        // Get file path
        val filePath = virtualFile?.path ?: ""

        // Get selected text
        val selectedText = selectionModel.selectedText ?: ""

        // Get line numbers (1-indexed)
        val lineStart = document.getLineNumber(selectionModel.selectionStart) + 1
        val lineEnd = document.getLineNumber(selectionModel.selectionEnd) + 1

        // Get workspace path
        val workspace = project?.basePath ?: ""

        // Find dependencies and tests in background
        ApplicationManager.getApplication().executeOnPooledThread {
            val (dependencies, totalCount, additionalCount) = if (project != null && psiFile != null) {
                findDependencies(project, psiFile, selectionModel.selectionStart, selectionModel.selectionEnd)
            } else {
                Triple(emptyList(), 0, 0)
            }

            val relatedTests = if (project != null && virtualFile != null) {
                findRelatedTests(project, virtualFile)
            } else {
                emptyList()
            }

            val payload = SelectionPayload(
                file_path = filePath,
                selected_text = selectedText,
                line_start = lineStart,
                line_end = lineEnd,
                workspace = workspace,
                dependencies = dependencies,
                dependency_count = totalCount,
                additional_dependency_count = additionalCount,
                related_tests = relatedTests
            )

            sendRequest(project, payload)
        }
    }

    private fun findDependencies(
        project: Project,
        psiFile: PsiFile,
        selectionStart: Int,
        selectionEnd: Int
    ): Triple<List<Dependency>, Int, Int> {
        return ReadAction.compute<Triple<List<Dependency>, Int, Int>, Throwable> {
            val allDependencies = mutableListOf<Dependency>()
            val seenLocations = mutableSetOf<String>()

            // Get all PSI elements in the selection
            val startElement = psiFile.findElementAt(selectionStart)
            val endElement = psiFile.findElementAt(selectionEnd)

            if (startElement != null && endElement != null) {
                // Find the common parent that contains the selection
                val commonParent = PsiTreeUtil.findCommonParent(startElement, endElement)

                if (commonParent != null) {
                    // Get all named elements (identifiers, references) in the selection
                    val elements = PsiTreeUtil.collectElements(commonParent) { element ->
                        val textRange = element.textRange
                        textRange != null &&
                            textRange.startOffset >= selectionStart &&
                            textRange.endOffset <= selectionEnd &&
                            element.children.isEmpty() // Leaf elements only
                    }

                    for (element in elements) {
                        // Find references to this element
                        val references = try {
                            ReferencesSearch.search(element, GlobalSearchScope.projectScope(project))
                                .findAll()
                        } catch (e: Exception) {
                            emptyList()
                        }

                        for (ref in references) {
                            val refElement = ref.element
                            val refFile = refElement.containingFile?.virtualFile ?: continue

                            // Skip references in the same file
                            if (refFile.path == psiFile.virtualFile?.path) continue

                            val refDocument = refElement.containingFile?.viewProvider?.document ?: continue
                            val refLine = refDocument.getLineNumber(refElement.textOffset) + 1

                            val locationKey = "${refFile.path}:$refLine"
                            if (seenLocations.contains(locationKey)) continue
                            seenLocations.add(locationKey)

                            allDependencies.add(Dependency(
                                file_path = refFile.path,
                                line = refLine
                            ))
                        }
                    }
                }
            }

            val totalCount = allDependencies.size

            // If more than MAX_DEPENDENCIES, return first 30 and count of additional
            if (totalCount > MAX_DEPENDENCIES) {
                Triple(allDependencies.take(MAX_DEPENDENCIES), totalCount, totalCount - MAX_DEPENDENCIES)
            } else {
                Triple(allDependencies.toList(), totalCount, 0)
            }
        }
    }

    private fun findRelatedTests(project: Project, virtualFile: VirtualFile): List<String> {
        return ReadAction.compute<List<String>, Throwable> {
            val relatedTests = mutableListOf<String>()
            val fileName = virtualFile.nameWithoutExtension

            // Language-agnostic test file patterns
            val testPatterns = listOf(
                // Standard patterns: file.test.ext, file.spec.ext
                "${fileName}.test.",
                "${fileName}.spec.",
                "${fileName}_test.",
                "${fileName}Test.",
                "${fileName}Tests.",
                "${fileName}Spec.",
                // Prefix patterns: test_file.ext, Test_file.ext
                "test_${fileName}.",
                "Test${fileName}."
            )

            // Excluded directories
            val excludedDirs = setOf(
                "node_modules", "bin", "obj", "target",
                ".venv", "venv", "__pycache__", "build", "dist"
            )

            val scope = GlobalSearchScope.projectScope(project)

            // Search for files matching test patterns
            for (pattern in testPatterns) {
                val matchingFiles = FilenameIndex.getAllFilesByExt(project, virtualFile.extension ?: "", scope)
                    .filter { file ->
                        file.name.startsWith(pattern.dropLast(1)) &&
                            !file.path.split("/").any { it in excludedDirs }
                    }
                    .take(10)

                for (file in matchingFiles) {
                    if (!relatedTests.contains(file.path)) {
                        relatedTests.add(file.path)
                    }
                }
            }

            // Also search in common test directories
            val testDirPatterns = listOf("tests", "test", "__tests__")
            for (testDir in testDirPatterns) {
                val testFiles = FilenameIndex.getFilesByName(
                    project,
                    "${fileName}.${virtualFile.extension}",
                    scope
                ).filter { psiFile ->
                    psiFile.virtualFile?.path?.contains("/$testDir/") == true &&
                        !psiFile.virtualFile?.path?.split("/")?.any { it in excludedDirs }!!
                }.mapNotNull { it.virtualFile?.path }

                for (path in testFiles) {
                    if (!relatedTests.contains(path)) {
                        relatedTests.add(path)
                    }
                }
            }

            relatedTests
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
