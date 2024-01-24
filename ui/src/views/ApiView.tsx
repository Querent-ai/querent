

import "swagger-ui-react/swagger-ui.css"
import SwaggerUI from 'swagger-ui-react'
import { ViewUnderAppBarBox, FullBoxContainer } from '../components/LayoutUtils';

function ApiView() {
    return <ViewUnderAppBarBox>
      <FullBoxContainer>
        <SwaggerUI
        layout="BaseLayout"
        defaultModelsExpandDepth={-1}
        url="/api-doc.json"/>
      </FullBoxContainer>
    </ViewUnderAppBarBox>
}

export default ApiView;
