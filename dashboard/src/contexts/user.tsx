import { createContext, PropsWithChildren, useEffect, useState } from "react";
import { useAPI, IUser } from "../services/api";



const defaultVal: IUser | null =  null
const UserContext = createContext<IUser | null>(null)

const UserProvider = (props: PropsWithChildren) => {

    const api = useAPI();
    const [user, setUser] = useState<IUser | null>(null);
    const [userFetched, setUserFetched] = useState<boolean>(false);

    useEffect(() => {
        api.getUser()
            .then((u) => setUser(u))
            .catch(() => setUser(null))
            .finally(() => setUserFetched(true))
    }, [])
    

    if (!userFetched) {
        return null;
    }

    return (
        <UserContext.Provider value={user}>
            {props.children}
        </UserContext.Provider>
    )

}


export {
    UserContext, 
    UserProvider,
}