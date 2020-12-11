import {commands, ExtensionContext, window, workspace} from 'vscode';
import {LanguageClient, LanguageClientOptions, ServerOptions} from 'vscode-languageclient';

let client: LanguageClient;

export function activate(_context: ExtensionContext) {
    let configuration = workspace.getConfiguration('lucent')
    let path: string = configuration.get('compiler.path');
    if (path.trim().length === 0) {
        let message = 'The Lucent compiler path has been not set. ';
        message += 'Specify the path in the extension settings.';
        let command = 'workbench.action.openSettings';
        window.showErrorMessage(message, 'Go').then(_ =>
            commands.executeCommand(command, 'lucent.compiler.path'));
        return;
    }

    let serverOptions: ServerOptions = {command: path, args: ['server']};
    let documentSelector = [{scheme: 'file', language: 'lucent'}];
    let clientOptions: LanguageClientOptions = {documentSelector};

    let client = new LanguageClient(
        'lucent-server',
        'Lucent Language Server',
        serverOptions,
        clientOptions,
    );

    client.registerProposedFeatures();
    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) return undefined;
    return client.stop();
}
