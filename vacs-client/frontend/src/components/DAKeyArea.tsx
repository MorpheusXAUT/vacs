import {useState} from "preact/hooks";
import Button from "./ui/Button.tsx";

function DAKeyArea() {
    const [contacts, _setContacts] = useState<string[]>(["LOVV_CTR", "LOVV_CTR",
        "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR", "LOVV_CTR"
    ]);

    return (
        <div className="flex flex-col flex-wrap h-full overflow-hidden py-3 px-2 gap-3 relative">
            {contacts.map(_contact =>
                <Button color="gray" className="w-25 h-[calc((100%-3.75rem)/6)] rounded !leading-4.5"><p>380<br/>B6<br/>EC</p></Button>
            )}
            {/*<div className="w-5 h-5 bg-red-500 absolute top-[50%]"></div> 320-340<br/>E2<br/>EC*/}
        </div>
    );
}

export default DAKeyArea;