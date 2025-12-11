import {useLayoutEffect, useMemo, useRef, useState} from "preact/hooks";
import {useEventCallback} from "./event-callback-hook.ts";

export const HEADER_HEIGHT_REM = 1.75;
const CALL_ROW_HEIGHT_REM = 2.7;

type UseListOptions = {
    itemsCount: number;
    selectedItem: number;
    setSelectedItem: (item: number) => void;
    defaultRows: number;
    enableKeyboardNavigation?: boolean;
};

export function useList({
    itemsCount,
    selectedItem,
    setSelectedItem,
    defaultRows,
    enableKeyboardNavigation,
}: UseListOptions) {
    const listContainer = useRef<HTMLDivElement>(null);
    const [listContainerHeight, setListContainerHeight] = useState<number>(0);
    const [scrollOffset, setScrollOffset] = useState<number>(0);

    const {visibleItemIndices, maxScrollOffset} = useMemo((): {
        visibleItemIndices: number[];
        maxScrollOffset: number;
    } => {
        let itemCount: number;

        if (listContainer.current) {
            const fontSize = parseFloat(getComputedStyle(listContainer.current).fontSize);
            const headerHeight = HEADER_HEIGHT_REM * fontSize;
            const itemHeight = CALL_ROW_HEIGHT_REM * fontSize;

            itemCount = Math.floor((listContainerHeight - headerHeight) / itemHeight);
        } else {
            itemCount = defaultRows;
        }

        return {
            visibleItemIndices: Array.from({length: itemCount}, (_, i) => scrollOffset + i),
            maxScrollOffset: itemsCount - itemCount,
        };
    }, [listContainerHeight, itemsCount, scrollOffset, defaultRows]);

    const onKeyDown = useEventCallback((event: KeyboardEvent) => {
        if (itemsCount === 0) return;

        if (event.key === "ArrowUp") {
            const firstVisibleCallIndex = visibleItemIndices[0];
            const newSelectedCall = Math.max(selectedItem - 1, 0);

            if (newSelectedCall < firstVisibleCallIndex) {
                setScrollOffset(scrollOffset => Math.max(scrollOffset - 1, 0));
            }

            setSelectedItem(newSelectedCall);
        } else if (event.key === "ArrowDown") {
            const lastVisibleCallIndex = visibleItemIndices[visibleItemIndices.length - 1];
            const newSelectedCall = Math.min(selectedItem + 1, itemsCount - 1);

            if (newSelectedCall > lastVisibleCallIndex) {
                setScrollOffset(scrollOffset => Math.min(scrollOffset + 1, maxScrollOffset));
            }

            setSelectedItem(newSelectedCall);
        }
    });

    useLayoutEffect(() => {
        if (!listContainer.current) return;
        const observer = new ResizeObserver(entries => {
            for (const entry of entries) {
                setListContainerHeight(entry.contentRect.height);
            }
        });
        observer.observe(listContainer.current);

        if (enableKeyboardNavigation === true) {
            window.addEventListener("keydown", onKeyDown);
        }

        return () => {
            observer.disconnect();

            if (enableKeyboardNavigation === true) {
                window.removeEventListener("keydown", onKeyDown);
            }
        };
    }, [onKeyDown, enableKeyboardNavigation]);

    return {listContainer, scrollOffset, setScrollOffset, visibleItemIndices, maxScrollOffset};
}
