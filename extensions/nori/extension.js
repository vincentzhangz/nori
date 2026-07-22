"use strict";

const vscode = require("vscode");
const prettier = require("prettier");

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
  const selector = { language: "nori", scheme: "*" };

  const formatter = vscode.languages.registerDocumentFormattingEditProvider(
    selector,
    {
      async provideDocumentFormattingEdits(document) {
        if (!vscode.workspace.getConfiguration("nori").get("format.enable", true)) {
          return [];
        }

        try {
          const formatted = await formatNoriSource(document.getText(), document);
          if (formatted === null || formatted === document.getText()) {
            return [];
          }
          const fullRange = new vscode.Range(
            document.positionAt(0),
            document.positionAt(document.getText().length)
          );
          return [vscode.TextEdit.replace(fullRange, formatted)];
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          vscode.window.showErrorMessage(`Nori format failed: ${message}`);
          return [];
        }
      }
    }
  );

  const rangeFormatter =
    vscode.languages.registerDocumentRangeFormattingEditProvider(selector, {
      async provideDocumentRangeFormattingEdits(document, range) {
        if (!vscode.workspace.getConfiguration("nori").get("format.enable", true)) {
          return [];
        }

        try {
          const fullText = document.getText();
          const formatted = await formatNoriSource(fullText, document, {
            rangeStart: document.offsetAt(range.start),
            rangeEnd: document.offsetAt(range.end)
          });
          if (formatted === null || formatted === fullText) {
            return [];
          }
          const fullRange = new vscode.Range(
            document.positionAt(0),
            document.positionAt(fullText.length)
          );
          return [vscode.TextEdit.replace(fullRange, formatted)];
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          vscode.window.showErrorMessage(`Nori format failed: ${message}`);
          return [];
        }
      }
    });

  const command = vscode.commands.registerCommand("nori.formatDocument", async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "nori") {
      vscode.window.showInformationMessage("Open a .nori file to format.");
      return;
    }
    await vscode.commands.executeCommand("editor.action.formatDocument");
  });

  context.subscriptions.push(formatter, rangeFormatter, command);
}

/**
 * @param {string} source
 * @param {vscode.TextDocument} document
 * @param {{ rangeStart?: number, rangeEnd?: number }} [options]
 * @returns {Promise<string | null>}
 */
async function formatNoriSource(source, document, options = {}) {
  const config = vscode.workspace.getConfiguration("nori");
  const printWidth = config.get("format.printWidth", 80);
  const tabWidth = config.get("format.tabWidth", 2);
  const singleQuote = config.get("format.singleQuote", false);
  const semi = config.get("format.semi", true);

  /** @type {import("prettier").Options} */
  const prettierOptions = {
    parser: "typescript",
    filepath: document.uri.fsPath.replace(/\.nori$/i, ".tsx"),
    printWidth,
    tabWidth,
    singleQuote,
    semi,
    trailingComma: "none",
    jsxSingleQuote: false
  };

  if (
    typeof options.rangeStart === "number" &&
    typeof options.rangeEnd === "number"
  ) {
    prettierOptions.rangeStart = options.rangeStart;
    prettierOptions.rangeEnd = options.rangeEnd;
  }

  return prettier.format(source, prettierOptions);
}

function deactivate() {}

module.exports = {
  activate,
  deactivate
};
