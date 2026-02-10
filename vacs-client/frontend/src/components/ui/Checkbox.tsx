import "../../styles/checkbox.css";
import {TargetedEvent} from "preact";
import {invokeSafe} from "../../error.ts";

type CheckboxProps = {
    name: string;
    checked: boolean;
    onChange?: (e: TargetedEvent<HTMLInputElement>) => void;
    muted?: boolean;
    disabled?: boolean;
};

function Checkbox(props: CheckboxProps) {
    return (
        <input
            type="checkbox"
            id={props.name}
            name={props.name}
            disabled={props.disabled}
            checked={props.checked}
            onChange={event => {
                if (props.muted !== true) {
                    void invokeSafe("audio_play_ui_click");
                }
                props.onChange?.(event);
            }}
            className="checkbox"
        />
    );
}

export default Checkbox;
