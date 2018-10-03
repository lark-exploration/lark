'use strict';

import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';

import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
} from 'vscode-languageclient';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
	// If the extension is launched in debug mode then the debug server options are used
	// Otherwise the run options are used

	var isWin = /^win/.test(process.platform);

	let serverPath;

	if (isWin) {
		serverPath = context.asAbsolutePath(path.join("..", "target", "debug", "lark.exe"));
	}
	else {
		serverPath = context.asAbsolutePath(path.join("..", "target", "debug", "lark"));
	}
	let serverOptions: ServerOptions = {
		run: { command: serverPath, args: ["ide"], options: { env: { "RUST_BACKTRACE": 1 } } },
		debug: {
			command: serverPath,
			args: ["ide"],
			options: { env: { "RUST_BACKTRACE": 1 } }
		}
	};

	// Options to control the language client
	let clientOptions: LanguageClientOptions = {
		// Register the server for plain text documents
		documentSelector: [{ scheme: 'file', language: 'lark' }],
		synchronize: {
			// Notify the server about file changes to '.clientrc files contained in the workspace
			fileEvents: workspace.createFileSystemWatcher('**/*.lark')
		}
	};

	// Create the language client and start the client.
	client = new LanguageClient(
		'LarkLSP',
		'Lark IDE support',
		serverOptions,
		clientOptions
	);

	// Start the client. This will also launch the server
	client.start();
}

export function deactivate(): Thenable<void> {
	if (!client) {
		return undefined;
	}
	return client.stop();
}
