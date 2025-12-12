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
    group: string | null
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

    const fetchJSON = async (url: string, init?: RequestInit) => {
        const authToken = auth.user?.access_token;

        if(!authToken) {
            throw Error("Using useAPI without an authenticated user is not possible")
        }

        const response = await fetch(
            url, 
            {
                method: init?.method ?? "GET",
                body: init?.body,
                headers: {
                    "Authorization": `Bearer ${authToken}`,
                    ...init?.headers,
                },
            }
        );

        if (!response.ok) {
            throw Error(`Request to "${response.url}" failed with status ${response.status}`);
        }

        const text = await response.text();
        if (!text) {
            return null;
        }

        return JSON.parse(text);
    }

    const getTasks = (filter: string | null, limit?: number, offset?: number): Promise<ITask[]> => {
        const urlParams = new URLSearchParams();
        if (filter) {
            urlParams.set("status", filter)
        }
        if (limit) {
            urlParams.set("limit", limit.toString())
        }
        if (offset) {
            urlParams.set("offset", offset.toString())
        }

        return fetchJSON(`${BASE_URL}/tasks?${urlParams.toString()}`);
    }

    const getTasksCount = (filter: string | null): Promise<{count: number}> => {
        return fetchJSON(`${BASE_URL}/tasks/count${filter ? `?status=${filter}` : ''}`);
    }

    const getTask = (id: string): Promise<ITask> => {
        return fetchJSON(`${BASE_URL}/tasks/${id}`);
    }

    const confirmTask = (id: string): Promise<ITask | null> => {
        return fetchJSON(`${BASE_URL}/tasks/${id}/confirm`, {
            method: "POST",
        });
    }

    const getTaskLogs = (id: string) => {
        return fetchJSON(`${BASE_URL}/tasks/${id}/logs`);
    }

    const getUser = (): Promise<IUser> => {
        return fetchJSON(`${BASE_URL}/user`);
    }
    
    
    return {
        getTasks,
        getTasksCount,
        getTask,
        confirmTask,
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
