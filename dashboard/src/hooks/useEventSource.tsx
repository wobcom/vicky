import { EventSourceMessage, fetchEventSource } from "@microsoft/fetch-event-source";
import { useCallback, useEffect, useRef, useState } from "react";
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

    const callbackRef = useRef(callback);
    useEffect(() => {
        callbackRef.current = callback;
    }, [callback]);

    const onMessage = useCallback((evt: EventSourceMessage) => {
        const x = evt.data;
        return callbackRef.current(x);
    }, [])

    useEffect(() => {
        if (!allowStart || !auth.user) {
            return;
        }

        const controller = new AbortController()

        const urlWithParam = params ? `${url}?start=${params.start}` : url;

        fetchEventSource(
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
        }
    }, [url, allowStart, auth.user, onMessage, params?.start])
    
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
