import {useProfileStore} from "../stores/profile-store.ts";
import DirectAccessPage from "../components/DirectAccessPage.tsx";
import GeoPage from "./GeoPage.tsx";

function MainPage() {
    const profile = useProfileStore(state => state.profile);
    const page = useProfileStore(state => state.page);

    return profile !== undefined ? (
        page !== undefined ? (
            <DirectAccessPage data={page} />
        ) : profile.geo !== undefined ? (
            <GeoPage page={profile.geo} />
        ) : profile.tabbed !== undefined ? (
            <p>TODO Tabbed Layout</p>
        ) : (
            <p>Unknown Profile</p>
        )
    ) : (
        <p>No Profile</p>
    );
}

export default MainPage;
