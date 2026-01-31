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
        ) : (
            <></>
        )
    ) : (
        <p>No Profile</p>
    );
}

export default MainPage;
