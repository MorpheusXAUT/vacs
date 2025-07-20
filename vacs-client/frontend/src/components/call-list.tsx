import {useState} from "preact/hooks";
import Button from "./ui/button.tsx";

function CallList() {
    const [contacts, _setContacts] = useState<string[]>(["LOVV_CTR", "LOVV_CTR",
        "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR"
    ]);

    return (
        <div className="flex flex-col flex-wrap h-full overflow-hidden p-2 gap-3 relative">
            {contacts.map(_contact =>
                <Button color="gray" className="w-26 h-[calc((100%-3.75rem)/6)] rounded !text-base">380<br/>B6<br/>EC</Button>
            )}
            {/*<div className="w-5 h-5 bg-red-500 absolute top-[50%]"></div> 320-340<br/>E2<br/>EC*/}
        </div>
    );
}

export default CallList;