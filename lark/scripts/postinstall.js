const shell = require("shelljs");
const { client } = require("./paths");

shell.exec("yarn", { cwd: client });
