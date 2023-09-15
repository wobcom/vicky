import axios, { Axios } from "axios"
import { useMemo } from "react"

type ITask = {
    id: string,
    display_name: string,
    locks: {
        type: "WRITE" | "READ"
        object: string,
    }[]
    status: {
        state: string,
        result?: string,
    }
}

type IUser = {
    full_name: String,
    role: "admin",
}

const useAPI = () => {

    const BASE_URL = "/api"

    const getTasks = (): Promise<ITask[]> => {
        return fetch(`${BASE_URL}/tasks`).then(x => x.json());
    }

    const getTaskLogs = (id: string) => {
        return fetch(`${BASE_URL}/tasks/${id}/logs`).then(x => x.json());
    }

    const getUser = (): Promise<IUser> => {
        return fetch(`${BASE_URL}/user`).then(x => x.json());
    }

    return {
        getTasks,
        getTaskLogs,
        getUser
    }

}

export {
    useAPI,
    ITask,
    IUser,
}