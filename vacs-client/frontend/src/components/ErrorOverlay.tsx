import {useErrorOverlayStore} from "../stores/error-overlay-store.ts";

function ErrorOverlay() {
    const visible = useErrorOverlayStore(state => state.visible);
    const title = useErrorOverlayStore(state => state.title);
    const message = useErrorOverlayStore(state => state.message);
    const close = useErrorOverlayStore(state => state.close);

    const handleClick = () => {
        close();
    };

    return visible ? (
        <div className="z-50 absolute top-0 left-0 w-full h-full flex justify-center items-center bg-[rgba(0,0,0,0.5)]" onClick={handleClick}>
            <div className="bg-gray-300 border-4 border-t-red-500 border-l-red-500 border-b-red-700 border-r-red-700 rounded w-100 py-2">
                <p className="w-full text-center text-lg font-semibold wrap-break-word">{title}</p>
                <p className="w-full text-center wrap-break-word">{message}</p>
            </div>
        </div>
    ) : <></>;
}

export default ErrorOverlay;