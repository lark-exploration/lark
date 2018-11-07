// npm install && cd client && npm run update-vscode && cd .. && npm run compile

const shell = require("shelljs");
const { root, client } = require("./paths");

shell.exec("yarn", { cwd: root });
shell.exec("yarn update-vscode", { cwd: client });
shell.exec("yarn compile", { cwd: root });