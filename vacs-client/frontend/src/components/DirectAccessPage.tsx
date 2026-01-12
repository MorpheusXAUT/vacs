import {DirectAccessPageDTO} from "../types/profile.ts";
import Button from "./ui/Button.tsx";
import {CSSProperties} from "preact";
import {clsx} from "clsx";

type DirectAccessPageProps = {
    data: DirectAccessPageDTO;
};

function DirectAccessPage({data}: DirectAccessPageProps) {
    const style: CSSProperties = {
        gridTemplateRows: `repeat(${data.rows}, 1fr)`,
        gridAutoFlow: "column",
    };

    return (
        <div className="w-full h-full overflow-auto">
            <div className="w-min min-h-full py-3 px-2 grid gap-2" style={style}>
                {data.keys.map((key, index) => (
                    <Button
                        key={index}
                        color="gray"
                        disabled={key.stationId === undefined}
                        className={clsx(
                            "w-25 h-full rounded p-1.5",
                            data.rows !== undefined && data.rows > 6
                                ? "leading-4!"
                                : "leading-4.5!",
                        )}
                    >
                        {key.label.map((s, index) => (
                            <p key={index}>{s}</p>
                        ))}
                    </Button>
                ))}
            </div>
        </div>
    );
}

export default DirectAccessPage;
