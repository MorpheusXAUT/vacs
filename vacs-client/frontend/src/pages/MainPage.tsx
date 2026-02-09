import {useProfileStore} from "../stores/profile-store.ts";
import DirectAccessPage from "../components/DirectAccessPage.tsx";
import GeoPage from "./GeoPage.tsx";
import FallbackProfile from "../components/FallbackProfile.tsx";

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

export default MainPage;
