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
import com.intellij.openapi.vcs.ProjectLevelVcsManager
import java.net.HttpURLConnection
import java.net.URI
import java.nio.charset.StandardCharsets
import com.google.gson.Gson
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.application.ReadAction
import com.intellij.codeInsight.daemon.impl.DaemonCodeAnalyzerImpl
import com.intellij.codeInsight.daemon.impl.HighlightInfo
import com.intellij.lang.annotation.HighlightSeverity

class SendSelectionAction : AnAction() {

    private data class Dependency(
        val file_path: String,
        val line: Int
    )

    private data class Diagnostic(
        /** Error, Warning, Information, or Hint */
        val severity: String,
        val message: String,
        val line: Int,
        val column: Int,
        /** Optional error code */
        val code: String? = null
    )

    private data class SelectionPayload(
        val file_path: String,
        val selected_text: String,
        val line_start: Int,
        val line_end: Int,
        val workspace: String,
        /** Git repository root if file is in a git repo, null otherwise */
        val git_root: String?,
        /** Project root: git_root > project_base_path > file's parent dir */
        val project_root: String,
        val dependencies: List<Dependency>,
        val dependency_count: Int,
        val additional_dependency_count: Int,
        val related_tests: List<String>,
        /** Errors and warnings from IDE analysis for this file */
        val diagnostics: List<Diagnostic>
    )

    companion object {
        private const val MAX_DEPENDENCIES = 30

        /**
         * Get the VCS (Git) root for a file.
         * Returns null if the file is not in a VCS repository.
         */
        fun getGitRoot(project: Project, virtualFile: VirtualFile?): String? {
            if (virtualFile == null) return null
            return try {
                val vcsManager = ProjectLevelVcsManager.getInstance(project)
                vcsManager.getVcsRootFor(virtualFile)?.path
            } catch (e: Exception) {
                null
            }
        }

        /**
         * Get the project root for a file with fallback chain:
         * 1. Git repository root (if in a git repo)
         * 2. Project base path
         * 3. Parent directory of the file
         */
        fun getProjectRoot(project: Project, virtualFile: VirtualFile?): String {
            // Try Git root first
            val gitRoot = getGitRoot(project, virtualFile)
            if (gitRoot != null) {
                return gitRoot
            }

            // Fall back to project base path
            val basePath = project.basePath
            if (basePath != null) {
                return basePath
            }

            // Last resort: parent directory of the file
            return virtualFile?.parent?.path ?: ""
        }
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

        // Get selected text and selection offsets on EDT before background thread
        val selectedText = selectionModel.selectedText ?: ""
        val selectionStart = selectionModel.selectionStart
        val selectionEnd = selectionModel.selectionEnd

        // Get line numbers (1-indexed)
        val lineStart = document.getLineNumber(selectionStart) + 1
        val lineEnd = document.getLineNumber(selectionEnd) + 1

        // Get workspace path
        val workspace = project?.basePath ?: ""

        // Find dependencies and tests in background
        ApplicationManager.getApplication().executeOnPooledThread {
            val (dependencies, totalCount, additionalCount) = if (project != null && psiFile != null) {
                findDependencies(project, psiFile, selectionStart, selectionEnd)
            } else {
                Triple(emptyList(), 0, 0)
            }

            val relatedTests = if (project != null && virtualFile != null) {
                findRelatedTests(project, virtualFile)
            } else {
                emptyList()
            }

            // Get diagnostics (errors, warnings) for the current file
            val diagnostics = if (project != null && psiFile != null) {
                findDiagnostics(project, psiFile)
            } else {
                emptyList()
            }

            // Get git root and project root for correct cwd in agent
            val gitRoot = if (project != null) getGitRoot(project, virtualFile) else null
            val projectRoot = if (project != null) getProjectRoot(project, virtualFile) else workspace

            val payload = SelectionPayload(
                file_path = filePath,
                selected_text = selectedText,
                line_start = lineStart,
                line_end = lineEnd,
                workspace = workspace,
                git_root = gitRoot,
                project_root = projectRoot,
                dependencies = dependencies,
                dependency_count = totalCount,
                additional_dependency_count = additionalCount,
                related_tests = relatedTests,
                diagnostics = diagnostics
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

    private fun findDiagnostics(project: Project, psiFile: PsiFile): List<Diagnostic> {
        return ReadAction.compute<List<Diagnostic>, Throwable> {
            val document = psiFile.viewProvider.document ?: return@compute emptyList()
            val diagnostics = mutableListOf<Diagnostic>()

            try {
                val highlights = DaemonCodeAnalyzerImpl.getHighlights(
                    document,
                    null,  // null = all severities
                    project
                )

                for (info in highlights) {
                    // Only include errors, warnings, and weak warnings
                    val severity = when {
                        info.severity >= HighlightSeverity.ERROR -> "Error"
                        info.severity >= HighlightSeverity.WARNING -> "Warning"
                        info.severity >= HighlightSeverity.WEAK_WARNING -> "Information"
                        else -> continue  // Skip hints and lower severities
                    }

                    val line = document.getLineNumber(info.startOffset) + 1
                    val lineStartOffset = document.getLineStartOffset(line - 1)
                    val column = info.startOffset - lineStartOffset + 1

                    diagnostics.add(Diagnostic(
                        severity = severity,
                        message = info.description ?: info.toolTip?.replace(Regex("<[^>]*>"), "") ?: "Unknown issue",
                        line = line,
                        column = column,
                        code = info.inspectionToolId
                    ))
                }
            } catch (e: Exception) {
                // If DaemonCodeAnalyzer is not available, return empty list
            }

            diagnostics
        }
    }

    private fun sendRequest(project: Project?, payload: SelectionPayload) {
        try {
            val url = URI("http://localhost:${KycoHttpAuth.port(payload.workspace)}/selection").toURL()
            val connection = url.openConnection() as HttpURLConnection

            connection.requestMethod = "POST"
            connection.setRequestProperty("Content-Type", "application/json")
            KycoHttpAuth.apply(connection, payload.workspace)
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
