import { useEffect, useMemo, useRef, useState } from "react"
import { getAnimationEnd } from "rsuite/esm/DOMHelper";
import { Terminal as XTerm } from "xterm"
import { FitAddon } from 'xterm-addon-fit';
import { useAPI } from "../services/api";

type TerminalProps = {
    taskId: string,
}

const Terminal = (props: TerminalProps) => {
    const name = useMemo(() => "terminal-" + Math.random().toString(36).substr(2, 8), [])
    const [ref, setRef] = useState<HTMLDivElement | null>(null);
    const [termRef, setTermRef] = useState<XTerm | null>(null);
    const openEventSource = useRef(false);

    const {taskId} = props;

    const api = useAPI();
    
    useEffect(() => {
        if (!termRef || openEventSource.current) {
            return
        }

        console.log("Foo")
        let evtSource = new EventSource(`/api/tasks/${taskId}/logs`);

        openEventSource.current = true;

        evtSource.onmessage = evt => {
            termRef.write(evt.data + "\r\n")
        }

    }, [termRef])




    useEffect(() => {
        if (ref && !termRef) {
            const iTerm = new XTerm()
            const fitAddon = new FitAddon();
            iTerm.loadAddon(fitAddon)
            iTerm.open(ref)
            setTermRef(iTerm)
            fitAddon.fit();
        }
    }, [ref])


    return (
        <div style={{width: "100%", height: "930px"}} id={name} ref={(ref) => setRef(ref)}></div>
    )

}

export {
    Terminal
}