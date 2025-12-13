package com.kyco.plugin

import java.io.File
import java.net.HttpURLConnection

object KycoHttpAuth {
    private const val AUTH_HEADER = "X-KYCO-Token"
    private const val DEFAULT_PORT = 9876

    fun apply(connection: HttpURLConnection, workspace: String?) {
        val token = workspace?.let { readTokenFromWorkspace(it) }
        if (!token.isNullOrBlank()) {
            connection.setRequestProperty(AUTH_HEADER, token)
        }
    }

    fun port(workspace: String?): Int {
        val parsed = workspace?.let { readPortFromWorkspace(it) }
        return parsed ?: DEFAULT_PORT
    }

    private fun readTokenFromWorkspace(workspace: String): String? {
        if (workspace.isBlank()) return null
        val text = readConfigText(workspace) ?: return null
        return extractHttpToken(text)
    }

    private fun readPortFromWorkspace(workspace: String): Int? {
        if (workspace.isBlank()) return null
        val text = readConfigText(workspace) ?: return null
        return extractHttpPort(text)
    }

    private fun readConfigText(workspace: String): String? {
        val configFile = File(workspace, ".kyco/config.toml")
        if (!configFile.exists() || !configFile.isFile) return null
        return configFile.readText()
    }

    private fun extractHttpToken(tomlText: String): String? {
        var inSettingsGui = false

        for (rawLine in tomlText.lineSequence()) {
            val line = rawLine.trim()
            if (line.isEmpty() || line.startsWith("#")) continue

            if (line.startsWith("[") && line.endsWith("]")) {
                inSettingsGui = line == "[settings.gui]"
                continue
            }

            if (!inSettingsGui) continue

            val mDouble = Regex("^http_token\\s*=\\s*\"([^\"]*)\"\\s*$").find(line)
            if (mDouble != null) {
                return mDouble.groupValues.getOrNull(1)?.takeIf { it.isNotEmpty() }
            }

            val mSingle = Regex("^http_token\\s*=\\s*'([^']*)'\\s*$").find(line)
            if (mSingle != null) {
                return mSingle.groupValues.getOrNull(1)?.takeIf { it.isNotEmpty() }
            }
        }

        return null
    }

    private fun extractHttpPort(tomlText: String): Int? {
        var inSettingsGui = false

        for (rawLine in tomlText.lineSequence()) {
            val line = rawLine.trim()
            if (line.isEmpty() || line.startsWith("#")) continue

            if (line.startsWith("[") && line.endsWith("]")) {
                inSettingsGui = line == "[settings.gui]"
                continue
            }

            if (!inSettingsGui) continue

            val m = Regex("^http_port\\s*=\\s*(\\d+)\\s*$").find(line) ?: continue
            val parsed = m.groupValues.getOrNull(1)?.toIntOrNull() ?: continue
            if (parsed in 1..65535) return parsed
        }

        return null
    }
}
