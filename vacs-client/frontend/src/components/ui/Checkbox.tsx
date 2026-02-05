import "../../styles/checkbox.css";
import {TargetedEvent} from "preact";

type CheckboxProps = {
    name: string;
    checked: boolean;
    onChange?: (e: TargetedEvent<HTMLInputElement>) => void;
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
            onChange={props.onChange}
            className="checkbox"
        />
    );
}

export default Checkbox;
