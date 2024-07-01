"use client"

import { useState } from "react"
import { useRouter } from "next/navigation"

import { Button } from "@/components/primitive/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/primitive/card"
import { InputOTP, InputOTPGroup, InputOTPSlot } from "@/components/primitive/otp-input"
import { Alert, AlertDescription } from "@/components/primitive/alert"

import {
  checkIfTwoFactorRequired,
  submitAuthFlow,
} from "@/lib/kratos/public/submit-auth-flow"

export type OtpParams = {
  flowId: string
  type: "login" | "register"
}

const OtpForm: React.FC<OtpParams> = ({ flowId, type }) => {
  const router = useRouter()
  const [otp, setOtp] = useState("")
  const [error, setError] = useState<string | null>(null)

  const submitOtpHandler = async (
    event: React.MouseEvent<HTMLButtonElement, MouseEvent>,
  ) => {
    event.preventDefault()
    if (otp.length !== 6) return
    setError(null)

    try {
      await submitAuthFlow({ flowId, otp, type })
    } catch (error) {
      console.error(error)
      setError("Invalid OTP or OTP has expired. Please go back and try again")
      return
    }

    const checkIfTwoFactorRequiredResponse = await checkIfTwoFactorRequired()
    if (checkIfTwoFactorRequiredResponse instanceof Error) {
      setError(checkIfTwoFactorRequiredResponse.message)
      return
    }

    if (
      checkIfTwoFactorRequiredResponse.userHasWebAuth &&
      checkIfTwoFactorRequiredResponse.userHasTotp
    )
      return router.replace(`/auth/2fa?flowId=${checkIfTwoFactorRequiredResponse.flowId}`)

    if (checkIfTwoFactorRequiredResponse.userHasTotp)
      return router.replace(
        `/auth/2fa/totp?flowId=${checkIfTwoFactorRequiredResponse.flowId}`,
      )

    if (checkIfTwoFactorRequiredResponse.userHasWebAuth)
      return router.replace(
        `/auth/2fa/webauthn?flowId=${checkIfTwoFactorRequiredResponse.flowId}`,
      )

    router.replace("/")
  }

  return (
    <Card variant="transparent" className="md:w-2/5">
      <CardHeader className="pt-4">
        <CardTitle>One time Password</CardTitle>
        <CardDescription className="text-textColor-secondary">
          An email has been sent to your email address. Please enter the OTP to continue.
        </CardDescription>
      </CardHeader>
      <form>
        <CardContent>
          <InputOTP value={otp} onChange={(value) => setOtp(value)} maxLength={6}>
            <InputOTPGroup className="w-full">
              <InputOTPSlot className="w-1/6 h-12" index={0} />
              <InputOTPSlot className="w-1/6 h-12" index={1} />
              <InputOTPSlot className="w-1/6 h-12" index={2} />
              <InputOTPSlot className="w-1/6 h-12" index={3} />
              <InputOTPSlot className="w-1/6 h-12" index={4} />
              <InputOTPSlot className="w-1/6 h-12" index={5} />
            </InputOTPGroup>
          </InputOTP>
        </CardContent>
        <CardFooter className="flex flex-col gap-2">
          <Button
            type="submit"
            className="rounded-full w-full"
            onClick={submitOtpHandler}
          >
            Submit
          </Button>
          {error && (
            <Alert variant="destructive" className="mt-1 p-3">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
        </CardFooter>
      </form>
    </Card>
  )
}

export { OtpForm }
