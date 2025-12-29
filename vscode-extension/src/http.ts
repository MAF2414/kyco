import * as vscode from 'vscode';
import * as http from 'http';
import type { SelectionPayload, BatchPayload, KycoHttpConfig } from './types';
import { KYCO_AUTH_HEADER, KYCO_DEFAULT_PORT } from './types';

export function sendSelectionRequest(payload: SelectionPayload, kycoHttp?: KycoHttpConfig): void {
    const jsonPayload = JSON.stringify(payload);

    const headers: Record<string, string | number> = {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(jsonPayload)
    };

    // Add auth token if available (required by kyco with http_token enabled)
    if (kycoHttp?.token) {
        headers[KYCO_AUTH_HEADER] = kycoHttp.token;
    }

    const options: http.RequestOptions = {
        hostname: 'localhost',
        port: kycoHttp?.port ?? KYCO_DEFAULT_PORT,
        path: '/selection',
        method: 'POST',
        headers,
        timeout: 5000
    };

    const req = http.request(options, (res) => {
        if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
            vscode.window.showInformationMessage('KYCo: Selection sent successfully');
        } else {
            vscode.window.showErrorMessage(`KYCo: Server responded with status ${res.statusCode}`);
        }
    });

    req.on('error', (err) => {
        vscode.window.showErrorMessage(`KYCo: Failed to send selection - ${err.message}`);
    });

    req.on('timeout', () => {
        req.destroy();
        vscode.window.showErrorMessage('KYCo: Request timed out');
    });

    req.write(jsonPayload);
    req.end();
}

export function sendBatchRequest(payload: BatchPayload, fileCount: number, kycoHttp?: KycoHttpConfig): void {
    const jsonPayload = JSON.stringify(payload);

    const headers: Record<string, string | number> = {
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(jsonPayload)
    };

    if (kycoHttp?.token) {
        headers[KYCO_AUTH_HEADER] = kycoHttp.token;
    }

    const options: http.RequestOptions = {
        hostname: 'localhost',
        port: kycoHttp?.port ?? KYCO_DEFAULT_PORT,
        path: '/batch',
        method: 'POST',
        headers,
        timeout: 5000
    };

    const req = http.request(options, (res) => {
        if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
            vscode.window.showInformationMessage(`KYCo: Batch sent (${fileCount} files)`);
        } else {
            vscode.window.showErrorMessage(`KYCo: Server responded with status ${res.statusCode}`);
        }
    });

    req.on('error', (err) => {
        vscode.window.showErrorMessage(`KYCo: Failed to send batch - ${err.message}`);
    });

    req.on('timeout', () => {
        req.destroy();
        vscode.window.showErrorMessage('KYCo: Request timed out');
    });

    req.write(jsonPayload);
    req.end();
}
