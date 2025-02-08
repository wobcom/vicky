import { useCallback, useEffect, useState } from "react"
import { ITask, useAPI } from "../services/api"
import { GlobalEvent, TaskUpdateEvent, useEventSource, useEventSourceJSON } from "./useEventSource";

const useTasks = (filter: string | null) => {
    const api = useAPI();
    const [tasks, setTasks] = useState<ITask[] | null>(null)
    
    const eventCallback = useCallback((evt: GlobalEvent) => {
        switch (evt.type) {
            case "TaskAdd": {
                api.getTasks(filter).then((tasks) => setTasks(tasks)); 
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
    }, [api])

    useEventSourceJSON<GlobalEvent>(`/api/events`, eventCallback)

    useEffect(() => {
        api.getTasks(filter).then((tasks) => setTasks(tasks));   
    }, [filter])

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
    }, [id])

    useEffect(() => {
        if (!id) {
            return;
        }

        api.getTask(id).then((task) => setTask(task));   
    }, [id])

    useEventSourceJSON<GlobalEvent>(`/api/events`, eventCallback)

    return task;
}


export {
    useTasks,
    useTask
}