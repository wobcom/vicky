import axios, { Axios } from "axios"
import { useMemo } from "react"


type Task = {
    id: string,
    display_name: string,
    status: {
        state: string,
        result?: string,
    }
}

type User = {
    full_name: String,
    role: "admin",
}

const useAPI = () => {

    const BASE_URL = "/api"

    const getTasks = (): Promise<Task[]> => {
        return fetch(`${BASE_URL}/tasks`).then(x => x.json());
    }

    const getTaskLogs = (id: string) => {
        return fetch(`${BASE_URL}/tasks/${id}/logs`).then(x => x.json());
    }

    const getUser = (): Promise<User> => {
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
    Task,
    User,
}