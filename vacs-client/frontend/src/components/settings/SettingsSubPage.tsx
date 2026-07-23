import {ComponentChildren} from "preact";
import {CloseButton} from "../../pages/SettingsPage.tsx";
import {clsx} from "clsx";
import {Submenu} from "../../stores/navigation-store.ts";

type SettingsSubPageProps = {
    title: string;
    width: string;
    actions?: ComponentChildren;
    className?: string;
    closeTargetSubmenu?: Submenu;
    children: ComponentChildren;
};

function SettingsSubPage(props: SettingsSubPageProps) {
    return (
        <div
            className={clsx(
                "absolute top-0 z-10 h-full bg-blue-700 border-t-0 px-2 pb-2 flex flex-col",
                props.width,
            )}
        >
            <p className="w-full text-white bg-blue-700 font-semibold text-center">{props.title}</p>
            <div className="w-full grow rounded-b-sm bg-[#B5BBC6] flex flex-col overflow-auto">
                <div className={clsx("w-full grow border-b-2 border-zinc-200", props.className)}>
                    {props.children}
                </div>
                <div
                    className={clsx(
                        "h-20 w-full shrink-0 flex flex-row gap-2 p-2 [&>button]:px-1 [&>button]:shrink-0 overflow-x-auto scrollbar-none",
                        props.actions !== undefined ? "justify-between" : "justify-end",
                    )}
                >
                    {props.actions}
                    <CloseButton
                        targetMenu={props.closeTargetSubmenu === undefined ? "settings" : undefined}
                        targetSubmenu={props.closeTargetSubmenu}
                    />
                </div>
            </div>
        </div>
    );
}

export default SettingsSubPage;
