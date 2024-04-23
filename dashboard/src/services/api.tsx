import axios, { Axios } from "axios"
import { useMemo } from "react"
import { useAuth } from "react-oidc-context"

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
    full_name: string,
    role: "admin",
}

const useAPI = () => {

    const BASE_URL = "/api"

    const auth = useAuth();

    const fetchJSON = async (url: string) => {
        const authToken = auth.user?.access_token;

        if(!authToken) {
            throw Error("Using useAPI without an authenticated user is not possible")
        }

        return fetch(
            url, 
            {
                headers: {
                    "Authorization": `Bearer ${authToken}`
                }
            }
        ).then(x => x.json());
    }

    const getTasks = (): Promise<ITask[]> => {
        return fetchJSON(`${BASE_URL}/tasks`);
    }

    const getTask = (id: string): Promise<ITask> => {
        return fetchJSON(`${BASE_URL}/tasks/${id}`);
    }

    const getTaskLogs = (id: string) => {
        return fetchJSON(`${BASE_URL}/tasks/${id}/logs`);
    }

    const getUser = (): Promise<IUser> => {
        return fetchJSON(`${BASE_URL}/user`);
    }

    return {
        getTasks,
        getTask,
        getTaskLogs,
        getUser
    }

}

export {
    useAPI,
    ITask,
    IUser,
}