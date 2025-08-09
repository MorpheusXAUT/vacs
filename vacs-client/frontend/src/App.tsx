import Clock from "./components/Clock.tsx";
import InfoGrid from "./components/InfoGrid.tsx";
import FunctionKeys from "./components/FunctionKeys.tsx";
import CallQueue from "./components/CallQueue.tsx";
import Button from "./components/ui/Button.tsx";
import {useEffect} from "preact/hooks";
import {invoke} from "@tauri-apps/api/core";
import {Route, Switch} from "wouter";
import LoginPage from "./pages/LoginPage.tsx";
import {useAuthStore} from "./stores/auth-store.ts";
import {setupAuthListeners} from "./listeners/auth-listener.ts";
import DAKeyArea from "./components/DAKeyArea.tsx";
import ConnectPage from "./pages/ConnectPage.tsx";
import SettingsPage from "./pages/SettingsPage.tsx";
import telephone from "./assets/telephone.svg";
import ErrorOverlay from "./components/ErrorOverlay.tsx";
import {invokeSafe} from "./error.ts";
import {setupErrorListener} from "./listeners/error-listener.ts";
import MissionPage from "./pages/MissionPage.tsx";
import TelephonePage from "./pages/TelephonePage.tsx";
import LinkButton from "./components/ui/LinkButton.tsx";
import {setupSignalingListeners} from "./listeners/signaling-listener.ts";
import {useSignalingStore} from "./stores/signaling-store.ts";
import PhoneButton from "./components/ui/PhoneButton.tsx";

function App() {
    const connected = useSignalingStore(state => state.connected);
    const authStatus = useAuthStore(state => state.status);

    useEffect(() => {
        void invoke("app_frontend_ready");

        setupErrorListener();
        setupAuthListeners();
        setupSignalingListeners();

        void invokeSafe("auth_check_session");
    }, []);

    return (
        <div className="h-screen flex flex-col">
            <div className="w-full h-12 bg-gray-300 flex flex-row border-gray-700 border-b">
                <Clock/>
                <InfoGrid/>
            </div>
            <div className="w-full h-[calc(100%-3rem)] flex flex-col">
                {/* Top Button Row */}
                <FunctionKeys/>
                <div className="flex flex-row w-full h-[calc(100%-10rem)] pl-1">
                    {/* Main Area */}
                    <div
                        className="h-full w-[calc(100%-6rem)] bg-[#B5BBC6] border-l-1 border-t-1 border-r-2 border-b-2 border-gray-700 rounded-sm flex flex-row">
                        <Switch>
                            <Route path="/">
                                {authStatus === "loading" ? (
                                    <></>
                                ) : authStatus === "unauthenticated" ? (
                                    <LoginPage/>
                                ) : connected ? (
                                    <DAKeyArea/>
                                ) : (
                                    <ConnectPage/>
                                )}
                            </Route>
                            <Route path="/settings" component={SettingsPage}/>
                            <Route path="/mission" component={MissionPage}/>
                            <Route path="/telephone" component={TelephonePage}/>
                        </Switch>
                    </div>
                    {/* Right Button Row */}
                    <div className="w-24 h-full px-2 pb-6 flex flex-col justify-between">
                        <LinkButton path="/telephone" className="h-16 shrink-0">
                            <img src={telephone} alt="Telephone" className="h-18 w-18" draggable={false} />
                        </LinkButton>
                        <CallQueue/>
                    </div>
                </div>
                {/* Bottom Button Row */}
                <div className="h-20 w-full p-2 pl-4 flex flex-row justify-between gap-20">
                    <div className="h-full flex flex-row gap-3">
                        <Button color="gray" className="text-xl w-46 text-gray-500" disabled={true}>Radio</Button>
                        <Button color="cyan" className="text-xl text-slate-400" disabled={true}>CPL</Button>
                        <Button color="cyan" className="text-xl w-46 text-slate-400" disabled={true}>
                            <p>RADIO<br/>PRIO</p>
                        </Button>
                        <PhoneButton/>
                    </div>
                    <Button color="cyan" className="text-xl w-44 px-10">END</Button>
                </div>
            </div>
            <ErrorOverlay />
        </div>
    );
}

export default App;
