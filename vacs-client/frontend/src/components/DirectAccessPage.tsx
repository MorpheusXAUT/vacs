import {DirectAccessPage as DirectAccessPageModel} from "../types/profile.ts";
import {CSSProperties} from "preact";
import DirectAccessStationKey from "./ui/DirectAccessStationKey.tsx";

type DirectAccessPageProps = {
    data: DirectAccessPageModel;
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
                    <DirectAccessStationKey
                        key={index}
                        data={key}
                        className={
                            data.rows !== undefined && data.rows > 6 ? "leading-4!" : "leading-4.5!"
                        }
                    />
                ))}
            </div>
        </div>
    );
}

export default DirectAccessPage;
