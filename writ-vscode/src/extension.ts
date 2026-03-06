import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient;

function getBinaryPath(
    context: vscode.ExtensionContext,
    name: string
): string {
    const binaryName = process.platform === 'win32' ? `${name}.exe` : name;
    const config = vscode.workspace.getConfiguration('writ');
    const override = config.get<string>('serverPath', '');
    if (override) {
        return path.join(override, binaryName);
    }
    return context.asAbsolutePath(path.join('bin', binaryName));
}

class WritDebugAdapterDescriptorFactory
    implements vscode.DebugAdapterDescriptorFactory {

    constructor(private context: vscode.ExtensionContext) {}

    createDebugAdapterDescriptor(
        _session: vscode.DebugSession
    ): vscode.ProviderResult<vscode.DebugAdapterDescriptor> {
        const binaryPath = getBinaryPath(this.context, 'writ-dap');
        return new vscode.DebugAdapterExecutable(binaryPath, []);
    }
}

export function activate(context: vscode.ExtensionContext): void {
    // --- LSP: bundled binary path (replaces Phase 53 dev path) ---
    const serverCommand = getBinaryPath(context, 'writ-lsp');

    if (!fs.existsSync(serverCommand)) {
        vscode.window.showErrorMessage(
            `Writ: language server binary not found at "${serverCommand}". ` +
            `Run the build script to bundle the binaries.`
        );
        return;
    }

    const serverOptions: ServerOptions = {
        command: serverCommand,
        args: [],
        options: { shell: false },
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'writ' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.writ'),
        },
    };

    client = new LanguageClient(
        'writ',
        'Writ Language Server',
        serverOptions,
        clientOptions
    );
    client.start();

    // --- DAP: register debug adapter descriptor factory ---
    const factory = new WritDebugAdapterDescriptorFactory(context);
    context.subscriptions.push(
        vscode.debug.registerDebugAdapterDescriptorFactory('writ', factory)
    );

    // --- DAP: trace logging (Output panel → "Writ DAP Trace") ---
    const dapLog = vscode.window.createOutputChannel('Writ DAP Trace');
    context.subscriptions.push(
        vscode.debug.registerDebugAdapterTrackerFactory('writ', {
            createDebugAdapterTracker(session: vscode.DebugSession) {
                const dapBinary = getBinaryPath(context, 'writ-dap');
                dapLog.appendLine(`[session ${session.id}] DAP binary: ${dapBinary}`);
                dapLog.appendLine(`[session ${session.id}] binary exists: ${fs.existsSync(dapBinary)}`);
                return {
                    onWillStartSession() {
                        dapLog.appendLine(`[session ${session.id}] >>> session starting`);
                    },
                    onWillReceiveMessage(message: unknown) {
                        dapLog.appendLine(`[client -> dap] ${JSON.stringify(message)}`);
                    },
                    onDidSendMessage(message: unknown) {
                        dapLog.appendLine(`[dap -> client] ${JSON.stringify(message)}`);
                    },
                    onError(error: Error) {
                        dapLog.appendLine(`[ERROR] ${error.message}`);
                    },
                    onWillStopSession() {
                        dapLog.appendLine(`[session ${session.id}] >>> session ending`);
                    },
                    onExit(code: number | undefined, signal: string | undefined) {
                        dapLog.appendLine(`[session ${session.id}] >>> exited code=${code} signal=${signal}`);
                    },
                };
            },
        })
    );
}

export function deactivate(): Thenable<void> | undefined {
    return client?.stop();
}
