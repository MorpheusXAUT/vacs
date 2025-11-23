import Button from "./ui/Button.tsx";
import wrenchAndDriver from "../assets/wrench-and-driver.svg";
import mission from "../assets/mission.svg";
import LinkButton from "./ui/LinkButton.tsx";

function FunctionKeys() {
    return (
        <div className="h-20 w-full flex flex-row gap-2 justify-between p-2 [&>button]:shrink-0">
            <Button color="cyan" className="text-slate-400" disabled={true}>PRIO</Button>
            <Button color="cyan" className="text-slate-400" disabled={true}>HOLD</Button>
            <Button color="cyan" className="text-slate-400" disabled={true}>PICKUP</Button>
            <Button color="cyan" className="text-slate-400" disabled={true}>
                <p>SUITE<br/>PICKUP</p>
            </Button>
            <Button color="cyan" className="text-slate-400" disabled={true}>TRANS</Button>
            <Button color="cyan" className="text-slate-400" disabled={true}>DIV</Button>
            <Button color="cyan" className="text-slate-400" disabled={true}>
                <p>PLAY<br/>BACK</p>
            </Button>
            <Button color="cyan" className="text-slate-400" disabled={true}>
                <p>PLC<br/>LSP<br/>on/off</p>
            </Button>
            <Button color="cyan" className="text-slate-400" disabled={true}>SPLIT</Button>
            <LinkButton path="/settings" className="h-full">
                <img src={wrenchAndDriver} alt="Settings" className="h-12 w-12" draggable={false}/>
            </LinkButton>
            <LinkButton path="/mission" className="h-full">
                <img src={mission} alt="Mission" className="h-14 w-14" draggable={false}/>
            </LinkButton>
        </div>
    );
}

export default FunctionKeys;