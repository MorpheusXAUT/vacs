import {JSX} from "preact";

type SelectProps = {
    selected: string;
    onChange: (value: string) => void;
    options: { value: string, text: string }[] | string[];
    className?: string;
    disabled?: boolean;
};

function Select(props: SelectProps) {
    const handleSelectChange = (event: JSX.TargetedEvent<HTMLSelectElement>) => {
        if (event.target instanceof HTMLSelectElement) {
            props.onChange(event.target.value);
        }
    }

    return (
        <select
            className="w-full border-2 border-t-gray-100 border-l-gray-100 border-r-gray-700 border-b-gray-700 rounded
            truncate mb-3 text-sm p-1 open:border-r-gray-100 open:border-b-gray-100 open:border-t-gray-700 open:border-l-gray-700"
            title="VoiceMeeter Aux Input (VB-Audio VoiceMeeter AUX VAIO)"
            onChange={handleSelectChange}
            value={props.selected}
            disabled={props.disabled}
        >
            { props.options.map(option => {
                if (typeof option === "string") {
                    return <option key={option} value={option}>{option}</option>;
                } else {
                    return <option key={option.value} value={option.value}>{option.text}</option>;
                }
            })}
        </select>
    );
}

export default Select;