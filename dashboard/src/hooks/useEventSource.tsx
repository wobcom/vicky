import { useCallback, useEffect, useRef } from "react";

export type LogEvent = String;

export type TaskAddEvent = {
    type: "TaskAdd"
}

export type TaskUpdateEvent = {
    type: "TaskUpdate",
    uuid: string,
}

export type GlobalEvent = TaskAddEvent | TaskUpdateEvent;

const useEventSource = (url: string, callback: (evt: string) => void, allowStart=true) => {
    const openEventSource = useRef<EventSource | null>(null);

    const onMessage = useCallback((evt: MessageEvent<any>) => {
        const x = evt.data;
        return callback(x);
    }, [callback])

    useEffect(() => {
        if(openEventSource.current) {
            openEventSource.current.onmessage = onMessage
        }
    }, [onMessage])

    useEffect(() => {
        if (!allowStart || openEventSource.current != null) {
            return;
        }

        let evtSource = new EventSource(url);

        openEventSource.current = evtSource;
        evtSource.onmessage = onMessage;
    
        return () => {
            evtSource.close()
            openEventSource.current = null;
        }
    }, [url, allowStart])
}

const useEventSourceJSON = <T extends any>(url: string, callback: (evt: T) => void) => {
    const onMessage = useCallback((evt: string) => {
        const x = JSON.parse(evt);
        return callback(x);
    }, [callback])


    return useEventSource(url, onMessage)
}

export {
    useEventSource,
    useEventSourceJSON
}