import axios, { Axios } from "axios"
import { useMemo } from "react"
import { useAuth } from "react-oidc-context"

type ITask = {
    id: string,
    display_name: string,
    locks: {
        type: "WRITE" | "READ"
        name: string,
    }[]
    status: {
        state: string,
        result?: string,
    }
    created_at: number,
    claimed_at: number | null,
    finished_at: number | null,
}

type IUser = {
    full_name: string,
    role: "admin",
}

type IWebConfig = {
    authority: string,
    client_id: string,
}

const BASE_URL = "/api"

const useAPI = () => {


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

    const getTasks = (filter: string | null): Promise<ITask[]> => {
        return fetchJSON(`${BASE_URL}/tasks${filter ? `?status=${filter}` : ''}`);
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
        getUser,
    }

}

const useUnauthenticatedAPI = () => {
    const fetchJSON = async (url: string) => {
        return fetch(
            url, 
        ).then(x => x.json());
    }

    const getWebConfig = (): Promise<IWebConfig> => {
        return fetchJSON(`${BASE_URL}/web-config`);
    }

    return {
        getWebConfig,
    }
}

export {
    useAPI,
    useUnauthenticatedAPI,
    ITask,
    IUser,
    IWebConfig,
}