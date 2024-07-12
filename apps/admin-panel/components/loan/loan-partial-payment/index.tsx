import { gql } from "@apollo/client"

import { useState } from "react"

import { toast } from "sonner"

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/primitive/dialog"
import { Input } from "@/components/primitive/input"
import { Label } from "@/components/primitive/label"
import { useLoanPartialPaymentMutation } from "@/lib/graphql/generated"
import { Button } from "@/components/primitive/button"
import { currencyConverter, formatCurrency } from "@/lib/utils"
import { DetailItem, DetailsGroup } from "@/components/details"

gql`
  mutation loanPartialPayment($input: LoanPartialPaymentInput!) {
    loanPartialPayment(input: $input) {
      loan {
        id
        loanId
        startDate
        balance {
          collateral {
            btcBalance
          }
          outstanding {
            usdBalance
          }
          interestIncurred {
            usdBalance
          }
        }
      }
    }
  }
`

export const LoanPartialPaymentDialog = ({
  loanId,
  refetch,
  children,
}: {
  loanId: string
  refetch?: () => void
  children: React.ReactNode
}) => {
  const [loanIdValue, setLoanIdValue] = useState<string>(loanId)
  const [amount, setAmount] = useState<number>(0)
  const [loanPartialPayment, { data, loading, error, reset }] =
    useLoanPartialPaymentMutation()

  const handlePartialPaymentSubmit = async () => {
    try {
      await loanPartialPayment({
        variables: {
          input: {
            amount: currencyConverter.usdToCents(amount),
            loanId,
          },
        },
      })

      toast.success("Payment successful")
      if (refetch) refetch()
    } catch (error) {
      console.error(error)
    }
  }

  return (
    <Dialog
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          setLoanIdValue(loanIdValue)
          setAmount(0)
          reset()
        }
      }}
    >
      <DialogTrigger asChild>{children}</DialogTrigger>
      {data ? (
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Payment Complete</DialogTitle>
            <DialogDescription>Loan details</DialogDescription>
          </DialogHeader>
          <DetailsGroup>
            <DetailItem label="Loan ID" value={data.loanPartialPayment.loan.loanId} />
            <DetailItem
              label="Start Date"
              value={data.loanPartialPayment.loan.startDate}
            />
            <DetailItem
              label="Collateral"
              value={`${data.loanPartialPayment.loan.balance.collateral.btcBalance} sats`}
            />
            <DetailItem
              label="Interest Incurred"
              value={formatCurrency({
                amount: currencyConverter.centsToUsd(
                  data.loanPartialPayment.loan.balance.interestIncurred.usdBalance,
                ),
                currency: "USD",
              })}
            />
            <DetailItem
              label="Outstanding"
              value={formatCurrency({
                amount: currencyConverter.centsToUsd(
                  data.loanPartialPayment.loan.balance.outstanding.usdBalance,
                ),
                currency: "USD",
              })}
            />
          </DetailsGroup>
        </DialogContent>
      ) : (
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Partial Payment Loan</DialogTitle>
            <DialogDescription>Fill in the details below.</DialogDescription>
          </DialogHeader>
          <div>
            <Label>Loan ID</Label>
            <Input
              type="text"
              value={loanIdValue}
              onChange={(e) => setLoanIdValue(e.target.value)}
              placeholder="Enter the user ID"
            />
          </div>
          <div>
            <Label>Amount</Label>
            <div className="flex items-center gap-1">
              <Input
                type="number"
                value={amount}
                onChange={(e) => setAmount(Number(e.target.value))}
                placeholder="Enter the desired principal amount"
                min={0}
              />
              <div className="p-1.5 bg-input-text rounded-md px-4">USD</div>
            </div>
          </div>
          {error && <span className="text-destructive">{error.message}</span>}
          <DialogFooter className="mt-4">
            <Button
              className="w-32"
              disabled={loading}
              onClick={handlePartialPaymentSubmit}
            >
              Submit
            </Button>
          </DialogFooter>
        </DialogContent>
      )}
    </Dialog>
  )
}