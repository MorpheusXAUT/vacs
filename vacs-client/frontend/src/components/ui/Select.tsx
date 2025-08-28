import {JSX} from "preact";
import {clsx} from "clsx";

export type SelectOption = { value: string, text: string, className?: string, hidden?: boolean, disabled?: boolean };

type SelectProps = {
    selected: string;
    onChange: (value: string) => void;
    options: SelectOption[];
    className?: string;
    disabled?: boolean;
};

function Select(props: SelectProps) {
    const title = props.options.find(option => option.value === props.selected)?.text;

    const handleSelectChange = (event: JSX.TargetedEvent<HTMLSelectElement>) => {
        event.preventDefault();
        if (event.target instanceof HTMLSelectElement) {
            props.onChange(event.target.value);
        }
    }

    return (
        <select
            className={clsx("w-full truncate mb-3 text-sm p-1 rounded",
                "bg-gray-300 border-2 border-t-gray-100 border-l-gray-100 border-r-gray-700 border-b-gray-700",
                "open:border-r-gray-100 open:border-b-gray-100 open:border-t-gray-700 open:border-l-gray-700",
                "disabled:text-gray-500",
                props.className)}
            title={title}
            onChange={handleSelectChange}
            value={props.selected}
            disabled={props.disabled}
        >
            {props.options.map(option =>
                <option key={option.value} value={option.value} className={option.className} hidden={option.hidden}
                        disabled={option.disabled}>{option.text}</option>
            )}
        </select>
    );
}

export default Select;