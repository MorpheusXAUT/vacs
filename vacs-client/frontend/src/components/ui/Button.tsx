import {clsx} from "clsx";
import {ComponentChildren} from "preact";

type ButtonColor = "gray" | "cyan" | "green" | "blue" | "emerald" | "red";
type ButtonHighlightColor = "green" | "gray";

export type ButtonProps = {
    color: ButtonColor;
    className?: string;
    children?: ComponentChildren;
    onClick?: (event: MouseEvent) => void;
    disabled?: boolean;
    hightlight?: ButtonHighlightColor;
};

const ButtonColors: Record<ButtonColor, string> = {
    cyan: "bg-[#92e1fe] border-t-cyan-100 border-l-cyan-100 border-r-cyan-950 border-b-cyan-950 rounded-md",
    green: "bg-[#4b8747] border-t-green-200 border-l-green-200 border-r-green-950 border-b-green-950 rounded-md",
    gray: "bg-gray-300 border-t-gray-100 border-l-gray-100 border-r-gray-700 border-b-gray-700 shadow-[0_0_0_1px_#364153] text-lg",
    blue: "bg-blue-700 border-t-blue-300 border-l-blue-300 border-r-blue-900 border-b-blue-900 text-white rounded-md",
    emerald: "bg-[#4b8747] border-t-green-200 border-l-green-200 border-r-green-950 border-b-green-950 rounded-md", // same background color as green, kept for separation of button types
    red: "bg-red-400 border-t-red-200 border-l-red-200 border-r-red-900 border-b-red-900 rounded-md",
};

const ActiveButtonColors: Record<ButtonColor, string> = {
    cyan: "active:border-r-cyan-100 active:border-b-cyan-100 active:border-t-cyan-950 active:border-l-cyan-950",
    green: "active:border-r-green-200 active:border-b-green-200 active:border-t-green-950 active:border-l-green-950",
    gray: "active:border-r-gray-100 active:border-b-gray-100 active:border-t-gray-700 active:border-l-gray-700",
    blue: "active:border-r-blue-300 active:border-b-blue-300 active:border-t-blue-900 active:border-l-blue-900",
    emerald: "active:border-r-green-200 active:border-b-green-200 active:border-t-green-950 active:border-l-green-950",
    red: "active:border-r-red-200 active:border-b-red-200 active:border-t-red-900 active:border-l-red-900",
};

export const DisabledButtonColors: Record<ButtonColor, string> = {
    cyan: "disabled:!border-cyan-900 disabled:!border",
    green: "disabled:!border-green-950 disabled:!border",
    gray: "disabled:!border-gray-700 disabled:!border disabled:!shadow-none",
    blue: "disabled:!border-blue-950 disabled:!border",
    emerald: "disabled:!border-emerald-950 disabled:!border",
    red: "disabled:!border-red-950 disabled:!border"
};

const ButtonHighlightColors: Record<ButtonHighlightColor, string> = {
    green: "bg-[#4b8747]",
    gray: "bg-gray-300"
};

function Button(props: ButtonProps) {
    return (
        <button
            className={clsx(
                "leading-5 w-20 cursor-pointer border-2 font-semibold",
                ButtonColors[props.color],
                ActiveButtonColors[props.color],
                DisabledButtonColors[props.color],
                props.className,
                props.hightlight !== undefined && "p-1.5",
            )}
            onClick={props.onClick}
            disabled={props.disabled}
        >
            {props.hightlight === undefined ? (
                props.children
            ) : (
                <div className={clsx(
                    "w-full h-full text-center flex items-center justify-center",
                    ButtonHighlightColors[props.hightlight],
                )}>
                    {props.children}
                </div>
            )}
        </button>
    );
}

export default Button;
