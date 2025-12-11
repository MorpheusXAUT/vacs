import {ComponentChildren} from "preact";
import Button from "./Button.tsx";
import {useLocation} from "wouter";
import {clsx} from "clsx";
import {navigate} from "wouter/use-browser-location";

type LinkButtonProps = {
    path: string;
    children: ComponentChildren;
    className?: string;
};

function LinkButton(props: LinkButtonProps) {
    const [location] = useLocation();

    const isActive = location.startsWith(props.path);

    return (
        <Button
            color={isActive ? "blue" : "cyan"}
            className={clsx("flex justify-center items-center", props.className)}
            onClick={() => navigate(isActive ? "/" : props.path)}
        >
            {props.children}
        </Button>
    );
}

export default LinkButton;
