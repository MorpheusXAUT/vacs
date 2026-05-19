import {clsx} from "clsx";
import {ComponentChildren} from "preact";
import {closeMenu, Menu, openMenu, useNavigationStore} from "../../stores/navigation-store.ts";
import Button from "./Button.tsx";

type LinkButtonProps = {
    menu: Menu;
    children: ComponentChildren;
    className?: string;
};

function LinkButton(props: LinkButtonProps) {
    const isActive = useNavigationStore(state => state.menu === props.menu);

    return (
        <Button
            color={isActive ? "blue" : "cyan"}
            className={clsx("flex justify-center items-center", props.className)}
            onClick={() => (isActive ? closeMenu() : openMenu(props.menu))}
        >
            {props.children}
        </Button>
    );
}

export default LinkButton;
