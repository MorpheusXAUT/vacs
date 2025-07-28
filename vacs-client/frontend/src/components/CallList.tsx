import Button from "./ui/Button.tsx";

function CallList() {
    // const position = 0.5;
    // const dragging = false;

    return (
        <div className="w-[35.75rem] h-full flex flex-col gap-3 p-3">
            <div className="w-full grow flex flex-row">
                <div className="grow h-full flex flex-col">
                    <div className="w-full h-8"></div>
                    <div className="w-full grow bg-yellow-50 border border-gray-500 border-r-0"></div>
                </div>
                <div className="h-full w-18 flex flex-col pt-8">
                    <div className="w-full h-12 border border-gray-500 border-b-0"></div>
                    <div className="relative grow w-full border border-gray-500 px-4 py-13">
                        <div className="h-full w-full border border-b-gray-100 border-r-gray-100 border-l-gray-700 border-t-gray-700 flex flex-col-reverse">
                            {/*<div className="w-full bg-blue-600" style={{height: `calc(100% * ${position})`}}></div>*/}
                        </div>
                        {/*<div*/}
                        {/*    className={clsx(*/}
                        {/*        "dotted-background absolute translate-y-[-50%] left-0 w-full aspect-square shadow-[0_0_0_1px_#364153] rounded-md cursor-pointer bg-blue-600 border",*/}
                        {/*        !dragging && "border-t-blue-200 border-l-blue-200 border-r-blue-900 border-b-blue-900",*/}
                        {/*        dragging && "border-b-blue-200 border-r-blue-200 border-l-blue-900 border-t-blue-900 shadow-none",*/}
                        {/*    )}*/}
                        {/*    style={{top: `calc(2.25rem + (1 - ${position}) * (100% - 4.5rem))`}}*/}
                        {/*/>*/}
                    </div>
                    <div className="w-full h-12 border border-gray-500 border-t-0"></div>
                </div>
            </div>
            <div className="w-full shrink-0 flex flex-row justify-between pr-18 [&_button]:h-15 [&_button]:rounded">
                <Button color="gray">
                    <p>Delete<br/>List</p>
                </Button>
                <Button color="gray" className="w-54">Call</Button>
            </div>
        </div>
    );
}

export default CallList;