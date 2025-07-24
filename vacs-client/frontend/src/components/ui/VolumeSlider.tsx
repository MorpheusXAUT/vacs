import {useEffect, useRef, useState} from "preact/hooks";
import type {JSX} from "preact";
import "../../styles/volume-slider.css";
import {clsx} from "clsx";

function VolumeSlider() {
    const [position, setPosition] = useState<number>(0.5);
    const [dragging, setDragging] = useState<boolean>(false);
    const isDragging = useRef<boolean>(false);
    const containerRef = useRef<HTMLDivElement>(null);

    function updatePositionFromClientY(clientY: number) {
        const container = containerRef.current;
        if (!container) return;

        const rect = container.getBoundingClientRect();
        const padding = 36; // 2.25rem = 36px padding (top and bottom)
        const usableHeight = rect.height - padding * 2;

        let newY = clientY - rect.top - padding;
        newY = Math.max(0, Math.min(newY, usableHeight));

        const newPos = 1 - newY / usableHeight;
        setPosition(newPos);
    }

    function handleMouseDown(event: MouseEvent | JSX.TargetedMouseEvent<HTMLDivElement>) {
        isDragging.current = true;
        setDragging(true);
        updatePositionFromClientY(event.clientY);
    }

    function handleMouseMove(event: MouseEvent) {
        if (!isDragging.current) return;
        updatePositionFromClientY(event.clientY);
    }

    function handleMouseUp() {
        isDragging.current = false;
        setDragging(false);
    }

    useEffect(() => {
        window.addEventListener("mouseup", handleMouseUp);
        window.addEventListener("mousemove", handleMouseMove);
        return () => {
            window.removeEventListener("mouseup", handleMouseUp);
            window.removeEventListener("mousemove", handleMouseMove);
        }
    })

    return (
        <div onMouseDown={handleMouseDown} ref={containerRef} className="relative h-full w-[4.75rem] my-2 mx-3 border-2 border-gray-500 rounded-lg px-4 py-10">
            <div className="h-full w-full border-2 border-b-gray-100 border-r-gray-100 border-l-gray-700 border-t-gray-700 flex flex-col-reverse">
                <div className="w-full bg-blue-600" style={{height: `calc(100% * ${position})`}}></div>
            </div>
            <div
                className={clsx(
                    "dotted-background absolute translate-y-[-50%] left-0 w-full aspect-square shadow-[0_0_0_1px_#364153] rounded-md cursor-pointer bg-blue-600 border-2",
                    !dragging && "border-t-blue-200 border-l-blue-200 border-r-blue-900 border-b-blue-900",
                    dragging && "border-b-blue-200 border-r-blue-200 border-l-blue-900 border-t-blue-900 shadow-none",
                )}
                style={{top: `calc(2.25rem + (1 - ${position}) * (100% - 4.5rem))`}}
            />
        </div>
    );
}

export default VolumeSlider;