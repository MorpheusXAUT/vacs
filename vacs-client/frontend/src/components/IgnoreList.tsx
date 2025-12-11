import List from "./ui/List.tsx";
import {clsx} from "clsx";
import Button from "./ui/Button.tsx";
import {useEffect, useState} from "preact/hooks";
import {TargetedEvent} from "preact";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";
import {invokeStrict} from "../error.ts";

function IgnoreList() {
    const [ignored, setIgnored] = useState<string[]>([]);
    const [selected, setSelected] = useState<number>(0);

    const [inputValue, setInputValue] = useState<string>("");

    const handleInputChange = (event: TargetedEvent<HTMLInputElement>) => {
        if (event.target instanceof HTMLInputElement) {
            const rawValue = event.target.value;

            const sanitized = rawValue
                .toUpperCase()
                .replace(/[^0-9]/g, "")
                .slice(0, 8);
            event.target.value = sanitized;

            setInputValue(sanitized);
        }
    };

    const handleAddClick = useAsyncDebounce(async () => {
        try {
            const added = await invokeStrict<boolean>("signaling_add_ignored_client", {
                clientId: inputValue,
            });
            if (added) setIgnored([...ignored, inputValue]);
            setInputValue("");
        } catch {}
    });

    const handleRemoveClick = useAsyncDebounce(async () => {
        const selectedCid = ignored[selected];
        try {
            await invokeStrict("signaling_remove_ignored_client", {clientId: selectedCid});
            setIgnored(ignored.filter(cid => cid !== selectedCid));
        } catch {}
    });

    useEffect(() => {
        const fetchIgnored = async () => {
            try {
                const ignored = await invokeStrict<string[]>("signaling_get_ignored_clients");
                setIgnored(ignored);
            } catch {}
        };

        void fetchIgnored();
    }, []);

    const ignoredRow = (index: number, isSelected: boolean, onClick: () => void) => {
        const color = isSelected ? "bg-blue-700 text-white" : "bg-yellow-50";

        return (
            <div
                className={clsx("px-0.5 flex items-center font-semibold", color)}
                onClick={onClick}
            >
                {ignored[index] ?? ""}
            </div>
        );
    };

    return (
        <div className="w-[25rem] h-full flex flex-col justify-between gap-2 p-3">
            <List
                className="w-full"
                itemsCount={ignored.length}
                selectedItem={selected}
                setSelectedItem={setSelected}
                defaultRows={10}
                row={ignoredRow}
                header={[{title: "Ignored CIDs"}]}
                columnWidths={["1fr"]}
                enableKeyboardNavigation={true}
            />
            <div className="w-full h-15 flex gap-2">
                <input
                    value={inputValue}
                    onChange={handleInputChange}
                    type="text"
                    className={clsx(
                        "flex-1 min-w-0 py-1 px-2.5 rounded-sm border border-gray-700 bg-slate-200 text-xl font-semibold",
                        "focus:border-blue-500 focus:outline-none",
                    )}
                />
                <Button
                    color="gray"
                    className="shrink-0 w-20"
                    onClick={handleAddClick}
                    disabled={inputValue === ""}
                >
                    Add
                </Button>
                <Button
                    color="gray"
                    className="shrink-0 w-20"
                    onClick={handleRemoveClick}
                    disabled={ignored[selected] === undefined || inputValue !== ""}
                >
                    Remove
                </Button>
            </div>
        </div>
    );
}

export default IgnoreList;
