import { Button, Col, FlexboxGrid, Panel, VStack } from "rsuite"

import GitHubIcon from '@rsuite/icons/legacy/Github';
import * as s from "./login.module.css";
import { useAuth } from "react-oidc-context";


const Login = () => {

    const auth = useAuth();

    return (
        <FlexboxGrid
            align="middle"
            justify="center"
        >
            <Col xs={8}>
                <Panel shaded bordered>
                    <VStack spacing={16} alignItems="center">
                        <h4>Anmeldung</h4>
                        <p className={s.ExplanationText}>
                            Es gibt verschiedene Möglichkeiten zum Einloggen. Bitte wählen Sie eine der unten genannten Möglichkeiten aus und
                            authentifizieren sie sich gegenüber Vicky.
                        </p>
                        <Button onClick={() => auth.signinRedirect()} color="violet" appearance="primary">
                            Login via OIDC
                        </Button>
                    </VStack>
                </Panel>
            </Col>

        </FlexboxGrid>
    )
}

export {
    Login
}