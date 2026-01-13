import Button from "../components/ui/Button.tsx";
import {
    GeoPageButton as GeoPageButtonModel,
    GeoPageContainer as GeoPageContainerModel,
    GeoPageDivider as GeoPageDividerModel,
    isGeoPageButton,
    isGeoPageContainer,
    isGeoPageDivider,
} from "../types/profile.ts";
import {CSSProperties} from "preact";
import {useProfileStore} from "../stores/profile-store.ts";
import DirectAccessPage from "../components/DirectAccessPage.tsx";

type GeoPageProps = {
    page: GeoPageContainerModel;
};

function GeoPage({page}: GeoPageProps) {
    const selectedPage = useProfileStore(state => state.page);

    return selectedPage !== undefined ? (
        <DirectAccessPage data={selectedPage} />
    ) : (
        <GeoPageContainer
            className="w-full h-full [&_div]:min-w-min [&_div]:min-h-min overflow-auto"
            container={page}
        />
    );
}

function GeoPageContainer({
    container,
    className,
}: {
    container: GeoPageContainerModel;
    className?: string;
}) {
    const style: CSSProperties = {
        height: container.height,
        width: container.width,
        display: "flex",
        flexDirection: container.direction === "row" ? "row" : "column",
        justifyContent: container.justifyContent,
        alignItems: container.alignItems,
        paddingLeft: container.paddingLeft && `${container.paddingLeft}rem`,
        paddingRight: container.paddingRight && `${container.paddingRight}rem`,
        paddingTop: container.paddingTop && `${container.paddingTop}rem`,
        paddingBottom: container.paddingBottom && `${container.paddingBottom}rem`,
        padding: container.padding && `${container.padding}rem`,
        gap: container.gap && `${container.gap}rem`,
    };

    return (
        <div className={className} style={style}>
            {container.children.map((child, index) => {
                if (isGeoPageContainer(child)) {
                    return <GeoPageContainer container={child} key={index} />;
                } else if (isGeoPageDivider(child)) {
                    return <GeoPageDivider divider={child} key={index} />;
                } else if (isGeoPageButton(child)) {
                    return <GeoPageButton button={child} key={index} />;
                }
            })}
        </div>
    );
}

type GeoPageButtonProps = {
    button: GeoPageButtonModel;
};

function GeoPageButton({button}: GeoPageButtonProps) {
    const setSelectedPage = useProfileStore(state => state.setPage);

    return (
        <Button
            color="gray"
            className={"aspect-square w-auto! rounded-none! overflow-hidden"}
            style={{height: `${button.size}rem`}}
            onClick={() => setSelectedPage(button.page)}
        >
            {button.label.map((s, index) => (
                <p key={index}>{s}</p>
            ))}
        </Button>
    );
}

type GeoPageDividerProps = {
    divider: GeoPageDividerModel;
};

// TODO: Make good
function GeoPageDivider({divider}: GeoPageDividerProps) {
    return (
        <div
            className="relative"
            style={{
                height: divider.orientation === "vertical" ? "100%" : `${divider.thickness}px`,
                width: divider.orientation === "horizontal" ? "100%" : `${divider.thickness}px`,
            }}
        >
            <div
                className="absolute -top-2 h-[calc(100%+1rem)] w-full"
                style={{backgroundColor: divider.color}}
            ></div>
        </div>
    );
}

export default GeoPage;
