const { app, BrowserWindow } = require("electron");
const path = require("path");
const { spawn } = require("child_process");

const isDev = !app.isPackaged;

let backendProcess = null;
let pythonProcess = null;

function startServices() {
    if (isDev) return;

    const resourcesPath = process.resourcesPath;

    const backendJar = path.join(resourcesPath, "backend", "backend-0.0.1-SNAPSHOT.jar");
    const pythonExecutable = path.join(resourcesPath, "python-core", "neatwork-python-core");

    pythonProcess = spawn(pythonExecutable, [], {
        stdio: "ignore",
        detatched: false
    });

    backendProcess = spawn("java", [
        "-jar",
        backendJar,
        "--spring.profiles.active=desktop",
    ], {
        stdio: "ignore",
        detached: false
    });
}

function createWindow() {
    const win = new BrowserWindow({
        width: 1440,
        height: 900,
        minWidth: 1100,
        minHeight: 700,
        backgroundColor: "#07111f",
        webPreferences: {
            nodeIntegration: false,
            contextIsolation: true
        }
    });

    if (isDev) {
        win.loadURL("http://localhost:5173");
    } else {
        win.loadFile(path.join(__dirname, "../dist/index.html"));
    }
}

app.whenReady().then(() => {
    startServices();

    setTimeout(() => {
        createWindow();
    }, 3000);
});

app.on("before-quit", () => {
    if (backendProcess) backendProcess.kill();
    if (pythonProcess) pythonProcess.kill();
});

app.on("window-all-closed", () => {
    if (process.platform != "darwin") app.quit();
});
