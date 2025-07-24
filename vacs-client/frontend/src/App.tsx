import Clock from "./components/Clock.tsx";
import InfoGrid from "./components/InfoGrid.tsx";
import TopButtonRow from "./components/TopButtonRow.tsx";
import IncomingList from "./components/IncomingList.tsx";
import Button from "./components/ui/Button.tsx";
import {useEffect} from "preact/hooks";
import {invoke} from "@tauri-apps/api/core";
import {Route, Switch} from "wouter";
import LoginPage from "./pages/LoginPage.tsx";
import {useAuthStore} from "./stores/auth-store.ts";
import {setupAuthListeners} from "./listeners/auth-listener.ts";
import CallList from "./components/CallList.tsx";
import ConnectPage from "./pages/ConnectPage.tsx";
import SettingsPage from "./pages/SettingsPage.tsx";

function App() {
    const authStatus = useAuthStore(state => state.status);

    useEffect(() => {
        void invoke("frontend_ready");

        setupAuthListeners();

        void invoke("check_auth_session");
    }, []);

    return (
        <div className="h-screen flex flex-col">
            <div className="w-full h-12 bg-gray-300 flex flex-row border-gray-700 border-b">
                <Clock/>
                <InfoGrid displayName="N36 PLC"/>
            </div>
            <div className="w-full h-[calc(100%-3rem)] flex flex-col">
                {/* Top Button Row */}
                <TopButtonRow/>
                <div className="flex flex-row w-full h-[calc(100%-10rem)] pl-1">
                    {/* Main Area */}
                    <div
                        className="h-full w-[calc(100%-6rem)] overflow-hidden bg-[#B5BBC6] border-l-1 border-t-1 border-r-2 border-b-2 border-gray-700 rounded-sm flex flex-row">
                        <Switch>
                            <Route path="/">
                                {authStatus === "unauthenticated" ? (
                                    <LoginPage/>
                                ) : (
                                    false ? ( // connected?
                                        <CallList/>
                                    ) : (
                                        <ConnectPage/>
                                    )
                                )}
                            </Route>
                            <Route path="/settings" component={SettingsPage}/>
                        </Switch>
                    </div>
                    {/* Right Button Row */}
                    <div className="w-24 h-full px-2 pb-6 flex flex-col justify-between">
                        <Button color="cyan" className="h-16 shrink-0"></Button>
                        <IncomingList/>
                    </div>
                </div>
                {/* Bottom Button Row */}
                <div className="h-20 w-full p-2 pl-4 flex flex-row justify-between gap-20">
                    <div className="h-full flex flex-row gap-3">
                        <Button color="emerald" className="text-xl w-46 font-semibold rounded-md">Radio</Button>
                        <Button color="cyan" className="text-xl">CPL</Button>
                        <Button color="cyan" className="text-xl w-46 text-slate-400" disabled={true}>
                            RADIO<br/>PRIO
                        </Button>
                        <Button color="gray"
                                className="w-46 min-h-16 !font-semibold !text-xl !rounded-md">Phone</Button>
                    </div>
                    <Button color="cyan" className="text-xl w-44 px-10">END</Button>
                </div>
            </div>
        </div>
    );
}

export default App;
