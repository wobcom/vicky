import { useCallback, useEffect, useState } from "react";
import { ITask, useAPI } from "../services/api";
import { GlobalEvent, useEventSourceJSON } from "./useEventSource";

const GROUP_FETCH_LIMIT = 400;

const extractGroups = (tasks: ITask[]) => {
    const unique = new Set<string>();
    tasks.forEach((task) => {
        if (task.group) {
            unique.add(task.group);
        }
    });
    return Array.from(unique).sort((a, b) => a.localeCompare(b));
};

const useTaskGroups = () => {
    const api = useAPI();
    const [groups, setGroups] = useState<string[]>([]);

    const refreshGroups = useCallback(() => {
        api.getTasks({ limit: GROUP_FETCH_LIMIT }).then((tasks) => setGroups(extractGroups(tasks)));
    }, [api]);

    const eventCallback = useCallback((evt: GlobalEvent) => {
        switch (evt.type) {
            case "TaskAdd":
            case "TaskUpdate": {
                refreshGroups();
                break;
            }
            default:
                break;
        }
    }, [refreshGroups]);

    useEffect(() => {
        refreshGroups();
    }, [refreshGroups]);

    useEventSourceJSON<GlobalEvent>(`/api/events`, eventCallback);

    return groups;
};

export {
    useTaskGroups,
}
