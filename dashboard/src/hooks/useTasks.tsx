import { useCallback, useEffect, useState } from "react"
import { ITask, useAPI } from "../services/api"
import { GlobalEvent, TaskUpdateEvent, useEventSourceJSON } from "./useEventSource";

const useTasksCount = (status: string | null, group: string | null) => {
    const api = useAPI();
    const [tasksCount, setTasksCount] = useState<number | null>(null)

    const refreshCount = useCallback(() => {
        api.getTasksCount({ status, group }).then((r) => setTasksCount(r.count));
    }, [api, status, group]);

    const eventCallback = useCallback((evt: GlobalEvent) => {
        switch (evt.type) {
            case "TaskAdd": {
                refreshCount();
                break;  
            }
            default: {
                break;
            }
        }
    }, [refreshCount])

    useEventSourceJSON<GlobalEvent>(`/api/events`, eventCallback)

    useEffect(() => {
        refreshCount();
    }, [refreshCount])

    return tasksCount;
}

const useTasks = (status: string | null, group: string | null, limit?: number, offset?: number) => {
    const api = useAPI();
    const [tasks, setTasks] = useState<ITask[] | null>(null)

    const fetchTasks = useCallback(() => {
        api.getTasks({ status, group, limit, offset }).then((tasks) => setTasks(tasks));
    }, [api, status, group, limit, offset]);
    
    const eventCallback = useCallback((evt: GlobalEvent) => {
        switch (evt.type) {
            case "TaskAdd": {
                fetchTasks()
                break;  
            }
            case "TaskUpdate": {
                const taskEvt = evt as TaskUpdateEvent;
                api.getTask(taskEvt.uuid).then((task) => {
                    setTasks(t => {
                        if (!t) {
                            return t;
                        }

                        const newTasks = [...t]
                        const tIndex = newTasks.findIndex(existingTask => existingTask.id === taskEvt.uuid)
                        if (tIndex < 0) {
                            return t;
                        }

                        newTasks[tIndex] = task
                        return newTasks;
                    })
                });   
                break;
            }
            default: {
                break;
            }
        }
    }, [api, fetchTasks])

    useEventSourceJSON<GlobalEvent>(`/api/events`, eventCallback)

    useEffect(() => {
        fetchTasks()
    }, [fetchTasks])

    return tasks;
}

const useTask = (id?: string | null) => {
    const api = useAPI();
    const [task, setTask] = useState<ITask | null>(null)
        
    const eventCallback = useCallback((evt: GlobalEvent) => {
        switch (evt.type) {
            case "TaskUpdate": {
                const taskEvt = evt as TaskUpdateEvent;

                if (id === taskEvt.uuid) {
                    api.getTask(id).then((task) => setTask(task));   
                }
                break;
            }
            default: {
                break;
            }
        }
    }, [api, id])

    useEffect(() => {
        if (!id) {
            return;
        }

        api.getTask(id).then((task) => setTask(task));   
    }, [api, id])

    useEventSourceJSON<GlobalEvent>(`/api/events`, eventCallback)

    return task;
}


export {
    useTasksCount,
    useTasks,
    useTask
}
