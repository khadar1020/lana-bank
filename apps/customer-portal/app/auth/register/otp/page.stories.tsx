import React from "react"

import RegisterOtp from "@/app/auth/register/otp/page"
import { AuthTemplateCard } from "@/components/auth/auth-template-card"
import { OtpForm } from "@/components/auth/otp-form"

export default {
  title: "pages/auth/register/otp",
  component: RegisterOtp,
}

export const Default = () => (
  <AuthTemplateCard>
    <OtpForm email="test@test.com" flowId="flow-id" formType="REGISTER" />
  </AuthTemplateCard>
)