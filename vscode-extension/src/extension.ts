import * as vscode from 'vscode';
import * as path from 'path';
import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
	// Get the server path from configuration
	const config = vscode.workspace.getConfiguration('delphi');
	const serverPath = config.get<string>('languageServer.path');

	if (!serverPath) {
		vscode.window.showErrorMessage('Delphi Language Server path is not configured. Please set delphi.languageServer.path in settings.');
		return;
	}

	// Create the server options
	const serverOptions: ServerOptions = {
		run: {
			command: serverPath,
			args: ['--lsp'],
			transport: TransportKind.stdio
		},
		debug: {
			command: serverPath,
			transport: TransportKind.stdio
		}
	};

	// Options to control the language client
	const clientOptions: LanguageClientOptions = {
		documentSelector: [{ scheme: 'file', language: 'delphi' }],
		synchronize: {
			fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{pas,dpr,dfm}')
		}
	};

	// Create and start the client
	client = new LanguageClient(
		'delphiLanguageServer',
		'Delphi Language Server',
		serverOptions,
		clientOptions
	);

	// Start the client and store the disposable
	client.start();
	context.subscriptions.push(client);
}

export function deactivate(): Thenable<void> | undefined {
	if (!client) {
		return undefined;
	}
	return client.stop();
}
