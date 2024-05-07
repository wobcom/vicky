import { createContext, PropsWithChildren, useEffect, useState } from "react";
import { useAPI, IUser, IWebConfig, useUnauthenticatedAPI } from "../services/api";

const defaultVal: IWebConfig | null =  null
const WebConfigContext = createContext<IWebConfig | null>(null)

const WebConfigProvider = (props: PropsWithChildren) => {

    const api = useUnauthenticatedAPI();
    const [webConfig, setWebConfig] = useState<IWebConfig | null>(null);
    const [webConfigFetched, setWebConfigFetched] = useState<boolean>(false);

    useEffect(() => {
        api.getWebConfig()
            .then((webConfig) => setWebConfig(webConfig))
            .catch((e) => {
                console.error(e)
                setWebConfig(null)
            })
            .finally(() => setWebConfigFetched(true))
    }, [])
    

    if (!webConfigFetched) {
        return null;
    }

    return (
        <WebConfigContext.Provider value={webConfig}>
            {props.children}
        </WebConfigContext.Provider>
    )

}


export {
    WebConfigContext, 
    WebConfigProvider,
}