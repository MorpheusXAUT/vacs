import {Link, useLocation} from "wouter";
import wrenchAndDriver from "../assets/wrench-and-driver.svg";
import Button from "./ui/button.tsx";
import {invoke} from "@tauri-apps/api/core";

function TopButtonRow() {
    const [location] = useLocation();

    return (
        <div className="h-20 w-full flex flex-row gap-2 justify-between p-2 [&>button]:px-1 [&>button]:shrink-0">
            <Button color="cyan">PRIO</Button>
            <Button color="cyan">HOLD</Button>
            <Button color="cyan">PICKUP</Button>
            <Button color="cyan">
                SUITE<br/>PICKUP
            </Button>
            <Button color="cyan">TRANS</Button>
            <Button color="cyan">DIV</Button>
            <Button color="cyan">
                PLAY<br/>BACK
            </Button>
            <Button color="cyan" className="text-slate-400" disabled={true}>
                PLC<br/>LSP<br/>on/off
            </Button>
            <Button color="cyan">SPLIT</Button>
            <Link to={location === "/settings" ? "/" : "/settings"}>
                <Button color={location === "/settings" ? "blue" : "cyan"} className="h-full flex justify-center items-center">
                    <img src={wrenchAndDriver} alt="Settings" className="h-12 w-12" />
                </Button>
            </Link>
            <Button color="cyan" className="min-w-20" onClick={() => invoke("greet")}></Button>
        </div>
    );
}

export default TopButtonRow;