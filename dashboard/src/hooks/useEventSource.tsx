import { EventSourceMessage, fetchEventSource } from "@microsoft/fetch-event-source";
import { MutableRefObject, Ref, RefCallback, useCallback, useEffect, useRef, useState } from "react";
import { useAuth } from "react-oidc-context"

export type LogEvent = String;

export type TaskAddEvent = {
    type: "TaskAdd"
}

export type TaskUpdateEvent = {
    type: "TaskUpdate",
    uuid: string,
}

export type GlobalEvent = TaskAddEvent | TaskUpdateEvent;

const useEventSource = (url: string, callback: (evt: string) => void, allowStart=true, params: ({start: number} | null) = null) => {

    const auth = useAuth();

    const openEventSource = useRef<Promise<void> | null>(null);
    const onMessage = useCallback((evt: EventSourceMessage) => {
        const x = evt.data;
        return callback(x);
    }, [callback])

    useEffect(() => {
        if (!allowStart || openEventSource.current != null || !auth.user) {
            return;
        }

        const controller = new AbortController()

        let urlWithParam = params ? `${url}?start=${params.start}` : url

        openEventSource.current = fetchEventSource(
            urlWithParam,
            {
                openWhenHidden: true,
                signal: controller.signal,
                headers: {
                    "Authorization": `Bearer ${auth.user.access_token}`
                },
                onmessage: onMessage
            }
        )

        return () => {
            controller.abort()
            openEventSource.current = null;
        }
    }, [url, allowStart, auth.user])

    
}

const useEventSourceJSON = <T extends any>(url: string, callback: (evt: T) => void) => {
    const onMessage = useCallback((evt: string) => {
        const x = JSON.parse(evt);
        return callback(x);
    }, [callback])


    return useEventSource(url, onMessage, true, null)
}

const useLogStream = (url: string, callback: (line: string) => void, allowStart: boolean) => {
    const [seenLogLines, setSeenLogLines] = useState(0);

    const onMessage = useCallback((str: string) => {
        setSeenLogLines(sSLL => sSLL + 1);
        return callback(str);
        
    }, [callback])

    return useEventSource(url, onMessage, allowStart, {start: seenLogLines})
}

export {
    useEventSource,
    useLogStream,
    useEventSourceJSON
}