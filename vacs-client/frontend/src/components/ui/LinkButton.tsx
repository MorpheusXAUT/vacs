import {ComponentChildren} from "preact";
import Button from "./Button.tsx";
import {Link, useLocation} from "wouter";
import {clsx} from "clsx";

type LinkButtonProps = {
    path: string;
    children: ComponentChildren;
    className?: string;
};

function LinkButton(props: LinkButtonProps) {
    const [location] = useLocation();

    return (
        <Link to={location === props.path ? "/" : props.path} draggable={false}>
            <Button
                color={location === props.path ? "blue" : "cyan"}
                className={clsx("flex justify-center items-center", props.className)}
            >
                {props.children}
            </Button>
        </Link>
    );
}

export default LinkButton;
