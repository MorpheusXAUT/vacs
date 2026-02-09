import {useProfileStore} from "../stores/profile-store.ts";
import DirectAccessPage from "../components/DirectAccessPage.tsx";
import GeoPage from "./GeoPage.tsx";
import ClientPage from "../components/ClientPage.tsx";
import {useSettingsStore} from "../stores/settings-store.ts";

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

export default MainPage;
