import {useMemo} from "react"
import {useAuth} from "react-oidc-context"

type ITask = {
    id: string,
    display_name: string,
    locks: {
        type: "WRITE" | "READ" | "CLEAN"
        name: string,
        poisoned: string,
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

type FilterParams = {
    status?: string | null;
    group?: string | null;
    limit?: number;
    offset?: number;
}

const useAPI = () => {


    const auth = useAuth();

    return useMemo(() => {
        const fetchJSON = async (url: string, init?: RequestInit) => {
            const authToken = auth.user?.access_token;

            if (!authToken) {
                throw Error("Using useAPI without an authenticated user is not possible");
            }

            const response = await fetch(url, {
                method: init?.method ?? "GET",
                body: init?.body,
                headers: {
                    Authorization: `Bearer ${authToken}`,
                    ...init?.headers,
                },
            });

            if (!response.ok) {
                throw Error(`Request to "${response.url}" failed with status ${response.status}`);
            }

            const text = await response.text();
            if (!text) {
                return null;
            }

            return JSON.parse(text);
        };

        const buildTaskParams = ({status, group, limit, offset}: FilterParams) => {
            const urlParams = new URLSearchParams();
            if (status) {
                urlParams.set("status", status);
            }
            if (group) {
                urlParams.set("group", group);
            }
            if (limit !== undefined) {
                urlParams.set("limit", limit.toString());
            }
            if (offset !== undefined) {
                urlParams.set("offset", offset.toString());
            }
            return urlParams;
        };

        const getTasks = (filter: FilterParams = {}): Promise<ITask[]> => {
            const urlParams = buildTaskParams(filter);
            const params = urlParams.toString();
            const query = params ? `?${params}` : "";

            return fetchJSON(`${BASE_URL}/tasks${query}`);
        };

        const getTasksCount = (filter: FilterParams = {}): Promise<{ count: number }> => {
            const urlParams = buildTaskParams(filter);
            const params = urlParams.toString();
            const query = params ? `?${params}` : "";
            return fetchJSON(`${BASE_URL}/tasks/count${query}`);
        };

        const getTask = (id: string): Promise<ITask> => {
            return fetchJSON(`${BASE_URL}/tasks/${id}`);
        };

        const confirmTask = (id: string): Promise<ITask | null> => {
            return fetchJSON(`${BASE_URL}/tasks/${id}/confirm`, {
                method: "POST",
            });
        };

        const abortTask = (id: string): Promise<ITask | null> => {
            return fetchJSON(`${BASE_URL}/tasks/${id}/cancel`, {
                method: "POST",
            });
        };

        const getTaskLogs = (id: string) => {
            return fetchJSON(`${BASE_URL}/tasks/${id}/logs`);
        };

        const getUser = (): Promise<IUser> => {
            return fetchJSON(`${BASE_URL}/user`);
        };

        return {
            getTasks,
            getTasksCount,
            getTask,
            abortTask,
            confirmTask,
            getTaskLogs,
            getUser,
        };
    }, [auth.user?.access_token]);
};

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
    FilterParams,
}
