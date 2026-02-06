import Button from "./Button.tsx";
import {useCallStore} from "../../stores/call-store.ts";
import {navigate} from "wouter/use-browser-location";
import {useFilterStore} from "../../stores/filter-store.ts";
import {useProfileStore, useProfileType} from "../../stores/profile-store.ts";
import {clsx} from "clsx";

function PhoneButton() {
    const blink = useCallStore(state => state.blink);
    const callDisplayType = useCallStore(state => state.callDisplay?.type);
    const setFilter = useFilterStore(state => state.setFilter);
    const setSelectedPage = useProfileStore(state => state.setPage);
    const navigateParentPage = useProfileStore(state => state.navigateParentPage);

    const isTabbedProfile = useProfileType() === "tabbed";

    return (
        <Button
            color={
                callDisplayType === "accepted"
                    ? "green"
                    : callDisplayType === "outgoing"
                      ? "gray"
                      : blink
                        ? callDisplayType === "error"
                            ? "red"
                            : "green"
                        : "gray"
            }
            highlight={
                callDisplayType === "outgoing" || callDisplayType === "rejected"
                    ? "green"
                    : undefined
            }
            className={clsx(
                "min-h-16 text-lg transition-[width]",
                isTabbedProfile ? "w-24" : "w-46",
            )}
            onClick={() => {
                setFilter("");
                if (isTabbedProfile) {
                    navigateParentPage();
                } else {
                    setSelectedPage(undefined);
                }
                navigate("/");
            }}
        >
            Phone
        </Button>
    );
}

export default PhoneButton;
