import {render} from "preact";
import App from "./App";
import "./styles/main.css";
import {error} from "@tauri-apps/plugin-log";
import {safeSerialize} from "./error.ts";

window.addEventListener("error", ev => {
    void error(
        `Webview error: ${JSON.stringify({
            filename: ev.filename,
            lineno: ev.lineno,
            colno: ev.colno,
            error: safeSerialize(ev.error),
        })}`,
    );
});
window.addEventListener("unhandledrejection", ev => {
    void error(`Unhandled webview rejection: ${JSON.stringify(safeSerialize(ev.reason))}`);
});

render(<App />, document.getElementById("root")!);
