import { gql } from "@apollo/client"
import React, { useEffect, useState } from "react"
import { toast } from "sonner"
import { useRouter } from "next/navigation"
import { PiPencilSimpleLineLight } from "react-icons/pi"

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/primitive/dialog"
import { Input } from "@/components/primitive/input"
import { Label } from "@/components/primitive/label"
import {
  InterestInterval,
  Period,
  useCreditFacilityCreateMutation,
  useTermsTemplatesQuery,
} from "@/lib/graphql/generated"
import { Button } from "@/components/primitive/button"
import { Select } from "@/components/primitive/select"
import { formatInterval, formatPeriod, currencyConverter } from "@/lib/utils"
import { DetailItem } from "@/components/details"

gql`
  mutation CreditFacilityCreate($input: CreditFacilityCreateInput!) {
    creditFacilityCreate(input: $input) {
      creditFacility {
        id
        creditFacilityId
      }
    }
  }
`

type CreateCreditFacilityDialogProps = {
  setOpenCreateCreditFacilityDialog: (isOpen: boolean) => void
  openCreateCreditFacilityDialog: boolean
  customerId: string
  refetch?: () => void
}

export const CreateCreditFacilityDialog: React.FC<CreateCreditFacilityDialogProps> = ({
  setOpenCreateCreditFacilityDialog,
  openCreateCreditFacilityDialog,
  customerId,
  refetch,
}) => {
  const router = useRouter()

  const [isCustomTerms, setIsCustomTerms] = useState(false)
  const { data: termsTemplatesData, loading: termsTemplatesLoading } =
    useTermsTemplatesQuery()
  const [createCreditFacility, { loading, error, reset }] =
    useCreditFacilityCreateMutation()
  const [useTemplateTerms, setUseTemplateTerms] = useState(true)
  const [selectedTemplateId, setSelectedTemplateId] = useState<string>("")

  const [formValues, setFormValues] = useState({
    facility: "0",
    annualRate: "",
    interval: "",
    liquidationCvl: "",
    marginCallCvl: "",
    initialCvl: "",
    durationUnits: "",
    durationPeriod: "",
  })

  useEffect(() => {
    if (
      termsTemplatesData?.termsTemplates &&
      termsTemplatesData.termsTemplates.length > 0
    ) {
      const latestTemplate = termsTemplatesData.termsTemplates[0]
      setSelectedTemplateId(latestTemplate.id)
      setFormValues((prevValues) => ({
        ...prevValues,
        annualRate: latestTemplate.values.annualRate.toString(),
        interval: latestTemplate.values.interval,
        liquidationCvl: latestTemplate.values.liquidationCvl.toString(),
        marginCallCvl: latestTemplate.values.marginCallCvl.toString(),
        initialCvl: latestTemplate.values.initialCvl.toString(),
        durationUnits: latestTemplate.values.duration.units.toString(),
        durationPeriod: latestTemplate.values.duration.period,
      }))
    }
  }, [termsTemplatesData])

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target
    setFormValues((prevValues) => ({
      ...prevValues,
      [name]: value,
    }))
    if (name === "facility") return
    setIsCustomTerms(true)
    setSelectedTemplateId("")
  }

  const handleTemplateChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const templateId = e.target.value
    setSelectedTemplateId(templateId)
    setIsCustomTerms(false)
    const selectedTemplate = termsTemplatesData?.termsTemplates.find(
      (t) => t.id === templateId,
    )
    if (selectedTemplate) {
      setFormValues((prevValues) => ({
        ...prevValues,
        annualRate: selectedTemplate.values.annualRate.toString(),
        interval: selectedTemplate.values.interval,
        liquidationCvl: selectedTemplate.values.liquidationCvl.toString(),
        marginCallCvl: selectedTemplate.values.marginCallCvl.toString(),
        initialCvl: selectedTemplate.values.initialCvl.toString(),
        durationUnits: selectedTemplate.values.duration.units.toString(),
        durationPeriod: selectedTemplate.values.duration.period,
      }))
    }
  }

  const handleCreateCreditFacility = async (event: React.FormEvent) => {
    event.preventDefault()
    const {
      facility,
      annualRate,
      interval,
      liquidationCvl,
      marginCallCvl,
      initialCvl,
      durationUnits,
      durationPeriod,
    } = formValues

    if (
      !facility ||
      !annualRate ||
      !interval ||
      !liquidationCvl ||
      !marginCallCvl ||
      !initialCvl ||
      !durationUnits ||
      !durationPeriod
    ) {
      toast.error("Please fill in all the fields.")
      return
    }

    try {
      await createCreditFacility({
        variables: {
          input: {
            customerId,
            facility: currencyConverter.usdToCents(Number(facility)),
            terms: {
              annualRate: parseFloat(annualRate),
              interval: interval as InterestInterval,
              liquidationCvl: parseFloat(liquidationCvl),
              marginCallCvl: parseFloat(marginCallCvl),
              initialCvl: parseFloat(initialCvl),
              duration: {
                units: parseInt(durationUnits),
                period: durationPeriod as Period,
              },
            },
          },
        },
        onCompleted: (data) => {
          toast.success("Credit Facility created successfully")
          if (refetch) refetch()
          router.push(
            `/credit-facilities/${data?.creditFacilityCreate.creditFacility.creditFacilityId}`,
          )
        },
      })
    } catch (err) {
      console.error(err)
    }
  }

  const resetForm = () => {
    setUseTemplateTerms(true)
    setIsCustomTerms(false)
    if (
      termsTemplatesData?.termsTemplates &&
      termsTemplatesData.termsTemplates.length > 0
    ) {
      const latestTemplate = termsTemplatesData.termsTemplates[0]
      setSelectedTemplateId(latestTemplate.id)
      setFormValues({
        facility: "0",
        annualRate: latestTemplate.values.annualRate.toString(),
        interval: latestTemplate.values.interval,
        liquidationCvl: latestTemplate.values.liquidationCvl.toString(),
        marginCallCvl: latestTemplate.values.marginCallCvl.toString(),
        initialCvl: latestTemplate.values.initialCvl.toString(),
        durationUnits: latestTemplate.values.duration.units.toString(),
        durationPeriod: latestTemplate.values.duration.period,
      })
    } else {
      setFormValues({
        facility: "0",
        annualRate: "",
        interval: "",
        liquidationCvl: "",
        marginCallCvl: "",
        initialCvl: "",
        durationUnits: "",
        durationPeriod: "",
      })
    }
  }

  const handleCloseDialog = () => {
    setOpenCreateCreditFacilityDialog(false)
    resetForm()
    reset()
  }

  useEffect(() => {
    if (openCreateCreditFacilityDialog) {
      resetForm()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [openCreateCreditFacilityDialog])

  return (
    <Dialog open={openCreateCreditFacilityDialog} onOpenChange={handleCloseDialog}>
      <DialogContent className="min-w-max">
        <DialogHeader>
          <DialogTitle>Create Credit Facility</DialogTitle>
          <DialogDescription>
            Fill in the details to create a credit facility.
          </DialogDescription>
        </DialogHeader>
        <form className="flex flex-col gap-4" onSubmit={handleCreateCreditFacility}>
          <div>
            <Label>Facility Amount</Label>
            <div className="flex items-center gap-1">
              <Input
                type="number"
                name="facility"
                value={formValues.facility}
                onChange={handleChange}
                placeholder="Enter the facility amount"
                min={0}
                required
              />
              <div className="p-1.5 bg-input-text rounded-md px-4">USD</div>
            </div>
          </div>
          {useTemplateTerms && (
            <div>
              <Label>Terms Template</Label>
              <Select
                value={selectedTemplateId}
                onChange={handleTemplateChange}
                disabled={termsTemplatesLoading}
              >
                <option value="" disabled selected={isCustomTerms}>
                  Select from predefined terms template
                </option>
                {termsTemplatesData?.termsTemplates.map((template) => (
                  <option key={template.id} value={template.id}>
                    {template.name}
                  </option>
                ))}
              </Select>
            </div>
          )}
          {useTemplateTerms ? (
            <>
              <div
                onClick={() => setUseTemplateTerms(false)}
                className="mt-2 flex items-center space-x-2 ml-2 cursor-pointer text-sm hover:underline w-fit"
              >
                <div>Credit Facility Terms</div>
                <PiPencilSimpleLineLight className="w-5 h-5 cursor-pointer text-primary" />
              </div>
              <div className="grid grid-cols-2 gap-x-2">
                <DetailItem
                  label="Interest Rate (APR)"
                  value={formValues.annualRate + "%"}
                />
                <DetailItem label="Initial CVL %" value={formValues.initialCvl} />
                <DetailItem
                  label="Duration"
                  value={
                    String(formValues.durationUnits) +
                    " " +
                    formatPeriod(formValues.durationPeriod as Period)
                  }
                />
                <DetailItem label="Margin Call CVL %" value={formValues.marginCallCvl} />
                <DetailItem
                  label="Payment Schedule"
                  className="space-x-7"
                  value={formatInterval(formValues.interval as InterestInterval)}
                />
                <DetailItem label="Liquidation CVL %" value={formValues.liquidationCvl} />
              </div>
            </>
          ) : (
            <>
              <div>
                <Label>Initial CVL (%)</Label>
                <Input
                  type="number"
                  name="initialCvl"
                  value={formValues.initialCvl}
                  onChange={handleChange}
                  placeholder="Enter the initial CVL"
                  required
                />
              </div>
              <div>
                <Label>Margin Call CVL (%)</Label>
                <Input
                  type="number"
                  name="marginCallCvl"
                  value={formValues.marginCallCvl}
                  onChange={handleChange}
                  placeholder="Enter the margin call CVL"
                  required
                />
              </div>
              <div>
                <Label>Liquidation CVL (%)</Label>
                <Input
                  type="number"
                  name="liquidationCvl"
                  value={formValues.liquidationCvl}
                  onChange={handleChange}
                  placeholder="Enter the liquidation CVL"
                  min={0}
                  required
                />
              </div>
              <div>
                <Label>Duration</Label>
                <div className="flex gap-2">
                  <Input
                    type="number"
                    name="durationUnits"
                    value={formValues.durationUnits}
                    onChange={handleChange}
                    placeholder="Duration"
                    min={0}
                    required
                    className="w-1/2"
                  />
                  <Select
                    name="durationPeriod"
                    value={formValues.durationPeriod}
                    onChange={handleChange}
                    required
                  >
                    <option value="" disabled>
                      Select period
                    </option>
                    {Object.values(Period).map((period) => (
                      <option key={period} value={period}>
                        {formatPeriod(period)}
                      </option>
                    ))}
                  </Select>
                </div>
              </div>
              <div>
                <Label>Interest Payment Schedule</Label>
                <Select
                  name="interval"
                  value={formValues.interval}
                  onChange={handleChange}
                  required
                >
                  <option value="" disabled>
                    Select interval
                  </option>
                  {Object.values(InterestInterval).map((interval) => (
                    <option key={interval} value={interval}>
                      {formatInterval(interval)}
                    </option>
                  ))}
                </Select>
              </div>
              <div>
                <Label>Annual Rate (%)</Label>
                <Input
                  type="number"
                  name="annualRate"
                  value={formValues.annualRate}
                  onChange={handleChange}
                  placeholder="Enter the annual rate"
                  required
                />
              </div>
            </>
          )}
          {error && <span className="text-destructive">{error.message}</span>}
          <DialogFooter className={!useTemplateTerms ? "sm:justify-between" : ""}>
            {!useTemplateTerms && (
              <div
                onClick={() => setUseTemplateTerms(true)}
                className="flex items-center space-x-2 cursor-pointer text-sm hover:underline"
              >
                Show less...
              </div>
            )}
            <Button
              onClick={handleCreateCreditFacility}
              className="w-48"
              disabled={loading}
              type="submit"
              loading={loading}
            >
              Create Credit Facility
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}