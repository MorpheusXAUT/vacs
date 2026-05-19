import {useNavigationStore} from "../stores/navigation-store.ts";
import SettingsPage from "./SettingsPage.tsx";
import MissionPage from "./MissionPage.tsx";
import TelephonePage from "./TelephonePage.tsx";
import PhonePage from "./PhonePage.tsx";
import RadioPage from "./RadioPage.tsx";

function Router() {
    const page = useNavigationStore(state => state.page);
    const menu = useNavigationStore(state => state.menu);

    const hidePage = menu === "settings" || menu === "mission";

    return (
        <>
            {menu === "settings" ? (
                <SettingsPage />
            ) : menu === "mission" ? (
                <MissionPage />
            ) : menu === "telephone" ? (
                <TelephonePage />
            ) : (
                <></>
            )}
            {!hidePage && (page === "phone" ? <PhonePage /> : <RadioPage />)}
        </>
    );
}

export default Router;
