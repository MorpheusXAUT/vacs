import Button from "../components/ui/Button.tsx";

function MainPage() {
    return (
        <>
            <div className="w-full h-full border-r-4 border-r-gray-500 flex flex-row">
                <div className="h-full p-2 flex flex-col justify-between">
                    <Button color="gray" className="text-lg !rounded-none w-30 h-30">KAR_MUN</Button>
                    <Button color="gray" className="text-lg !rounded-none w-30 h-30">KAR_MUN</Button>
                    <Button color="gray" className="text-lg !rounded-none w-30 h-30">ZUR</Button>
                </div>
                <div className="w-full flex flex-col">
                    <div className="w-full flex flex-row justify-between p-2">
                        <Button color="gray" className="text-lg !rounded-none w-30 h-30">FIC</Button>
                        <Button color="gray" className="text-lg !rounded-none w-30 h-30">Cont</Button>
                        <Button color="gray" className="text-lg !rounded-none w-30 h-30">FDU</Button>
                    </div>
                    <div className="h-full w-full flex flex-col">
                        <div className="py-1 h-full w-full flex flex-row gap-3 items-end justify-center">
                            <Button color="gray" className="text-lg !rounded-none w-50 h-50">
                                <span className="block">B</span>
                                <span className="block">LOWS</span>
                            </Button>
                            <Button color="gray" className="text-lg !rounded-none w-50 h-50">
                                <span className="block">N</span>
                                <span className="block">LOWL</span>
                            </Button>
                            <Button color="gray" className="text-lg !rounded-none w-50 h-50">
                                <span className="block">E</span>
                                <span className="block">APP</span>
                            </Button>
                        </div>
                        <div className="py-1 h-full w-full flex flex-row gap-3 items-start justify-center">
                            <Button color="gray" className="text-lg !rounded-none w-50 h-50">
                                <span className="block">W</span>
                                <span className="block">WI_WK</span>
                            </Button>
                            <Button color="gray" className="text-lg !rounded-none w-50 h-50">
                                <span className="block">S</span>
                                <span className="block">LOWG</span>
                            </Button>
                        </div>
                    </div>
                    <div className="w-full flex flex-row justify-between px-20 py-2">
                        <Button color="gray" className="text-lg !rounded-none w-30 h-30">PAD</Button>
                        <Button color="gray" className="text-lg !rounded-none w-30 h-30">LJU</Button>
                    </div>
                </div>
                <div className="h-full p-2 pr-4 flex flex-col justify-between">
                    <Button color="gray" className="text-lg !rounded-none w-30 h-30">PRA</Button>
                    <Button color="gray" className="text-lg !rounded-none w-30 h-30">BRA</Button>
                    <Button color="gray" className="text-lg !rounded-none w-30 h-30">BUD</Button>
                    <Button color="gray" className="text-lg !rounded-none w-30 h-30">ZAG</Button>
                </div>
            </div>
            <div className="h-full p-2 pl-4 flex flex-col justify-between">
                <Button color="gray" className="text-lg !rounded-none w-30 h-30">MIL</Button>
                <Button color="gray" className="text-lg !rounded-none w-30 h-30">FMP</Button>
                <Button color="gray" className="text-lg !rounded-none w-30 h-30">SUP</Button>
                <Button color="gray" className="text-lg !rounded-none w-30 h-30">CWP</Button>
            </div>
        </>
    );
}

export default MainPage;