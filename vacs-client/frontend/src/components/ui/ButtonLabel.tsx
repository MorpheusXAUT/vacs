type ButtonLabelProps = {
    label: string[];
};

function ButtonLabel({label}: ButtonLabelProps) {
    return label.map((s, index) => (
        <p key={index} className="max-w-full truncate" title={s}>
            {s}
        </p>
    ));
}

export default ButtonLabel;
