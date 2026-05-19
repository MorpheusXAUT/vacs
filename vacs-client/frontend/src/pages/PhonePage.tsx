import LoginPage from "./LoginPage.tsx";
import ConnectPage from "./ConnectPage.tsx";
import {useConnectionStore} from "../stores/connection-store.ts";
import {useAuthStore} from "../stores/auth-store.ts";
import {useProfileStore} from "../stores/profile-store.ts";
import DirectAccessPage from "../components/DirectAccessPage.tsx";
import GeoPage from "./GeoPage.tsx";
import {useSettingsStore} from "../stores/settings-store.ts";
import ClientPage from "../components/ClientPage.tsx";

function PhonePage() {
    const connected = useConnectionStore(state => state.connectionState === "connected");
    const testing = useConnectionStore(state => state.connectionState === "test");
    const authStatus = useAuthStore(state => state.status);

    return authStatus === "loading" ? (
        <></>
    ) : authStatus === "unauthenticated" && !testing ? (
        <LoginPage />
    ) : connected || testing ? (
        <MainPage />
    ) : (
        <ConnectPage />
    );
}

function MainPage() {
    const profile = useProfileStore(state => state.profile);
    const page = useProfileStore(state => state.page.current);

    return profile !== undefined ? (
        page !== undefined ? (
            <DirectAccessPage data={page} />
        ) : profile.geo !== undefined ? (
            <GeoPage page={profile.geo} />
        ) : (
            <></>
        )
    ) : (
        <FallbackProfile />
    );
}

function FallbackProfile() {
    const config = useSettingsStore(state => state.selectedClientPageConfig);

    return (
        <div className="w-full h-full overflow-auto">
            <div className="w-min min-h-full py-3 px-2 grid grid-flow-col grid-rows-6 gap-2">
                <ClientPage config={config} />
            </div>
        </div>
    );
}

export default PhonePage;
