import { useWindowSize } from "@uidotdev/usehooks";
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { Terminal as XTerm } from "xterm"
import { FitAddon } from 'xterm-addon-fit';
import { useEventSource, useLogStream } from "../hooks/useEventSource";
import { useAPI } from "../services/api";

type TerminalProps = {
    taskId: string,
}

const Terminal = (props: TerminalProps) => {
    const name = useMemo(() => "terminal-" + Math.random().toString(36).substr(2, 8), [])
    const [ref, setRef] = useState<HTMLDivElement | null>(null);
    const fitRef = useRef<FitAddon | null>(null);
    const [termRef, setTermRef] = useState<XTerm | null>(null);
    const openEventSource = useRef(false);

    const {taskId} = props;

    const api = useAPI();
    const size = useWindowSize();

    const eventCallback = useCallback((logLine: string) => {
        if (!termRef) {
            return
        }

        termRef.write(logLine + "\r\n")
    }, [termRef])

    useLogStream(`/api/tasks/${taskId}/logs`, eventCallback, termRef != null);
    
    useEffect(() => {
        // Window Resizing takes some more time than JS execution...
        setTimeout(() => fitRef.current?.fit(), 0)
    }, [size])

    useEffect(() => {
        if (ref && !termRef) {
            const iTerm = new XTerm({
                scrollback: 100000,
                fontFamily: "Mono"
            })
            const fitAddon = new FitAddon();
            iTerm.loadAddon(fitAddon)
            iTerm.open(ref)

            iTerm.element!.style.padding = "1em";

            setTermRef(iTerm)
            fitAddon.fit();
            fitRef.current = fitAddon;
        }
    }, [ref])


    return (
        <div style={{width: "100%", height: "calc( 100% - 48px )"}} id={name} ref={(ref) => setRef(ref)}></div>
    )

}

export {
    Terminal
}